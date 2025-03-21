use std::fs;
use std::path::Path;
use schedule_parser::parse_xls;

fn main() {
    let (teachers, groups) = parse_xls(Path::new("./schedule.xls"));

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
