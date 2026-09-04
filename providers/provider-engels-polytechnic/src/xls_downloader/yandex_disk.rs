use super::{FetchError, FetchResult, RemoteFile};
use chrono::{DateTime, Utc};
use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;

/// Prefix of the schedule file names in the shared folder.
const NAME_PREFIX: &str = "poltavskaja_";

/// Marker of the corrections file, which holds a separate schedule.
const NAME_EXCLUDED_MARKER: &str = "korr";

/// Extension of the schedule files.
const NAME_SUFFIX: &str = ".xls";

/// Maximum amount of entries requested from the folder listing.
const LISTING_LIMIT: u32 = 200;

/// Characters not allowed inside a single path segment of the public file link.
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}');

#[derive(Deserialize)]
struct Listing {
    #[serde(rename = "_embedded")]
    embedded: Embedded,
}

#[derive(Deserialize)]
struct Embedded {
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(rename = "type")]
    resource_type: String,
    name: String,
    modified: DateTime<Utc>,
    md5: Option<String>,
    revision: Option<u64>,
    file: Option<String>,
}

impl Item {
    /// Whether the entry is the schedule the provider is interested in.
    fn is_schedule(&self) -> bool {
        if self.resource_type != "file" || self.file.is_none() {
            return false;
        }

        let name = self.name.to_lowercase();

        name.starts_with(NAME_PREFIX)
            && name.ends_with(NAME_SUFFIX)
            && !name.contains(NAME_EXCLUDED_MARKER)
    }

    /// Marker changing whenever the file content changes.
    fn version(&self) -> String {
        self.md5
            .clone()
            .or_else(|| self.revision.map(|revision| revision.to_string()))
            .unwrap_or_else(|| self.modified.to_rfc3339())
    }
}

/// Finds the freshest schedule file in the public folder.
///
/// The files inside the folder are replaced independently of the folder link,
/// so the whole listing is re-read on every probe.
pub async fn probe(public_url: &str) -> FetchResult<RemoteFile> {
    let listing = super::get(&format!(
        "https://cloud-api.yandex.net/v1/disk/public/resources?public_key={}&limit={}",
        utf8_percent_encode(public_url, NON_ALPHANUMERIC),
        LISTING_LIMIT
    ))
    .await?
    .json::<Listing>()
    .await
    .map_err(|error| FetchError::unknown(std::sync::Arc::new(error)))?;

    let item = listing
        .embedded
        .items
        .into_iter()
        .filter(Item::is_schedule)
        .max_by_key(|item| (item.modified, item.revision))
        .ok_or(FetchError::NoScheduleFile)?;

    Ok(RemoteFile {
        url: format!(
            "{}/{}",
            public_url.trim_end_matches('/'),
            utf8_percent_encode(&item.name, PATH_SEGMENT)
        ),
        version: item.version(),
        modified_at: item.modified,
        download_url: item.file.unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use super::probe;

    const PUBLIC_URL: &str = "https://disk.yandex.ru/d/e8HJpMgDq7msyg";

    #[tokio::test]
    async fn probe_ok() {
        let file = probe(PUBLIC_URL).await.unwrap();

        assert!(file.url.starts_with(PUBLIC_URL));
        assert!(!file.version.is_empty());
        assert!(file.download_url.starts_with("https://"));
    }

    #[tokio::test]
    async fn probe_unknown_folder() {
        assert!(
            probe("https://disk.yandex.ru/d/000000000000000")
                .await
                .is_err()
        );
    }
}
