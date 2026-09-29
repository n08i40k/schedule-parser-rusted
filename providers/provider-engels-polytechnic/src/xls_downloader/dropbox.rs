use super::{FetchError, FetchResult, RemoteFile};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::Deserialize;
use std::sync::Arc;

/// Endpoint the Dropbox web interface lists shared folder entries with.
const LISTING_URL: &str = "https://www.dropbox.com/list_shared_link_folder_entries";

/// CSRF token sent both as a cookie and as a form field.
///
/// Dropbox only checks that the two values match, so any token is accepted.
const CSRF_TOKEN: &str = "schedule-parser-rusted";

/// Parts of a shared folder link (`https://www.dropbox.com/scl/fo/<key>/<hash>?rlkey=<rlkey>`)
/// required for the folder listing.
#[derive(Debug, PartialEq)]
struct FolderLink {
    link_key: String,
    secure_hash: String,
    rlkey: String,
}

impl FolderLink {
    fn parse(public_url: &str) -> FetchResult<Self> {
        let url = Url::parse(public_url).map_err(|_| FetchError::InvalidUrl)?;

        if !url
            .host_str()
            .is_some_and(|host| host == "dropbox.com" || host.ends_with(".dropbox.com"))
        {
            return Err(FetchError::InvalidUrl);
        }

        let segments: Vec<&str> = url
            .path_segments()
            .map(|segments| segments.filter(|segment| !segment.is_empty()).collect())
            .unwrap_or_default();

        let ["scl", "fo", link_key, secure_hash] = segments.as_slice() else {
            return Err(FetchError::InvalidUrl);
        };

        let rlkey = url
            .query_pairs()
            .find(|(key, _)| key == "rlkey")
            .map(|(_, value)| value.into_owned())
            .ok_or(FetchError::InvalidUrl)?;

        Ok(Self {
            link_key: link_key.to_string(),
            secure_hash: secure_hash.to_string(),
            rlkey,
        })
    }
}

#[derive(Deserialize)]
struct Listing {
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    filename: String,
    is_dir: bool,
    href: Option<String>,
    revision_id: Option<String>,
    ts: Option<i64>,
}

impl Entry {
    /// Whether the entry is the schedule the provider is interested in.
    fn is_schedule(&self) -> bool {
        !self.is_dir && self.href.is_some() && super::is_schedule_name(&self.filename)
    }

    fn modified_at(&self) -> DateTime<Utc> {
        self.ts
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .unwrap_or_default()
    }
}

/// Link to the direct download of a file, built from its preview link.
fn direct_download_url(href: &str) -> FetchResult<String> {
    let mut url = Url::parse(href).map_err(|_| FetchError::InvalidUrl)?;

    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "dl")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    url.query_pairs_mut()
        .clear()
        .extend_pairs(pairs)
        .append_pair("dl", "1");

    Ok(url.to_string())
}

/// Finds the freshest schedule file in the public folder.
///
/// The files inside the folder are replaced independently of the folder link,
/// so the whole listing is re-read on every probe.
pub async fn probe(public_url: &str) -> FetchResult<RemoteFile> {
    let link = FolderLink::parse(public_url)?;

    let response = reqwest::Client::new()
        .post(LISTING_URL)
        .header("User-Agent", ua_generator::ua::spoof_chrome_ua())
        .header("Cookie", format!("t={CSRF_TOKEN}"))
        .form(&[
            ("is_xhr", "true"),
            ("t", CSRF_TOKEN),
            ("link_key", &link.link_key),
            ("link_type", "c"),
            ("secure_hash", &link.secure_hash),
            ("sub_path", ""),
            ("rlkey", &link.rlkey),
        ])
        .send()
        .await
        .map_err(|error| FetchError::unknown(Arc::new(error)))?;

    if response.status().as_u16() != 200 {
        return Err(FetchError::bad_status_code(response.status().as_u16()));
    }

    let listing = response
        .json::<Listing>()
        .await
        .map_err(|error| FetchError::unknown(Arc::new(error)))?;

    let entry = listing
        .entries
        .into_iter()
        .filter(Entry::is_schedule)
        .max_by_key(|entry| entry.ts)
        .ok_or(FetchError::NoScheduleFile)?;

    let modified_at = entry.modified_at();
    let href = entry.href.unwrap();

    Ok(RemoteFile {
        download_url: direct_download_url(&href)?,
        version: entry
            .revision_id
            .unwrap_or_else(|| modified_at.to_rfc3339()),
        modified_at,
        url: href,
    })
}

#[cfg(test)]
mod tests {
    use super::{FolderLink, direct_download_url, probe};

    const PUBLIC_URL: &str = "https://www.dropbox.com/scl/fo/rkcg08aekpdat5sjg0er8/AOESrLwGCK3apE4oGqS4F_k?rlkey=jrv7d1kvq3lc6b7jxuj3uek41&st=uz87ct9v&dl=0";

    #[test]
    fn parse_link() {
        assert_eq!(
            FolderLink::parse(PUBLIC_URL).unwrap(),
            FolderLink {
                link_key: "rkcg08aekpdat5sjg0er8".to_string(),
                secure_hash: "AOESrLwGCK3apE4oGqS4F_k".to_string(),
                rlkey: "jrv7d1kvq3lc6b7jxuj3uek41".to_string(),
            }
        );

        assert!(FolderLink::parse("https://www.dropbox.com/scl/fo/key/hash").is_err());
        assert!(FolderLink::parse("https://disk.yandex.ru/d/e8HJpMgDq7msyg").is_err());
    }

    #[test]
    fn download_url() {
        assert_eq!(
            direct_download_url("https://www.dropbox.com/scl/fo/a/b/f.xls?rlkey=c&dl=0").unwrap(),
            "https://www.dropbox.com/scl/fo/a/b/f.xls?rlkey=c&dl=1"
        );
    }

    #[tokio::test]
    async fn probe_ok() {
        let file = probe(PUBLIC_URL).await.unwrap();

        assert!(file.url.starts_with("https://www.dropbox.com/scl/fo/"));
        assert!(!file.version.is_empty());
        assert!(file.download_url.ends_with("dl=1"));
    }

    #[tokio::test]
    async fn probe_unknown_folder() {
        assert!(
            probe("https://www.dropbox.com/scl/fo/000000000000000000000/AAAAAAAAAAAAAAAAAAAAAAA?rlkey=0000000000000000000000000")
                .await
                .is_err()
        );
    }
}
