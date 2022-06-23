use crate::vrs::Requirement;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path;

pub fn read_gomod_version<P: AsRef<path::Path>>(
    gomod_path: P,
) -> anyhow::Result<Option<Requirement>> {
    let file: fs::File;
    match fs::File::open(gomod_path) {
        Ok(f) => file = f,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => {
                return Ok(None);
            }
            _ => {
                return Err(err.into());
            }
        },
    };
    let mut version_option: Option<String> = None;
    for line_result in BufReader::new(file).lines() {
        let line = line_result?;
        let mut parts = line.trim().split_whitespace();
        match (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) {
            (Some("//"), Some("+heroku"), Some("goVersion"), Some(cmp), Some(vrs)) => {
                version_option = Some(format!("{cmp} {vrs}"));
                break;
            }
            (Some("//"), Some("+heroku"), Some("goVersion"), Some(vrs), None) => {
                version_option = Some(vrs.to_string());
                break;
            }
            (Some("go"), Some(vrs), None, None, None) => {
                version_option = Some(format!("={vrs}"));
            }
            _ => (),
        }
    }

    match version_option {
        None => Ok(None),
        Some(version_string) => Requirement::parse_go(&version_string).map(Some),
    }
}
