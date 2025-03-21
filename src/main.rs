use crate::xls_downloader::basic_impl::BasicXlsDownloader;
use crate::xls_downloader::interface::XLSDownloader;
use schedule_parser::parse_xls;
use std::{env, fs};

mod xls_downloader;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    assert_ne!(args.len(), 1);

    let mut downloader = BasicXlsDownloader::new();

    downloader
        .set_url(args[1].to_string())
        .await
        .expect("Failed to set url");

    let fetch_res = downloader.fetch(false).await.expect("Failed to fetch xls");

    let (teachers, groups) = parse_xls(fetch_res.data.as_ref().unwrap());

    fs::write(
        "./schedule.json",
        serde_json::to_string_pretty(&groups)
            .expect("Failed to serialize schedule")
            .as_bytes(),
    )
    .expect("Failed to write schedule");

    fs::write(
        "./teachers.json",
        serde_json::to_string_pretty(&teachers)
            .expect("Failed to serialize teachers schedule")
            .as_bytes(),
    )
    .expect("Failed to write teachers schedule");
}
