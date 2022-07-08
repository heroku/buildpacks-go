use crate::vrs::Requirement;
use anyhow::{Context, Result};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path;

pub struct GoModCfg {
    pub version: Option<Requirement>,
    pub packages: Option<Vec<String>>,
}

pub fn read_gomod_cfg<P: AsRef<path::Path>>(gomod_path: P) -> Result<GoModCfg> {
    let mut cfg = GoModCfg {
        version: None,
        packages: None,
    };
    let file = fs::File::open(gomod_path).context("failed to open go.mod")?;
    for line_result in BufReader::new(file).lines() {
        let line = line_result?;
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some("//"), Some("+heroku"), Some("install"), Some(pkg)) => {
                let mut pkgs = vec![pkg.to_string()];
                for pkgn in parts {
                    pkgs.push(pkgn.to_string());
                }
                cfg.packages = Some(pkgs);
            }
            (Some("//"), Some("+heroku"), Some("goVersion"), Some(vrs)) => {
                cfg.version = Requirement::parse_go(vrs).map(Some)?;
            }
            (Some("go"), Some(vrs), None, None) => {
                if cfg.version == None {
                    cfg.version = Requirement::parse_go(&format!("={vrs}")).map(Some)?;
                }
            }
            _ => (),
        }
    }
    Ok(cfg)
}
