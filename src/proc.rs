use libcnb::data::launch::{Launch, ProcessBuilder, ProcessType, ProcessTypeError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchErr {
    #[error("Invalid Go package name: {0}")]
    PackageName(String),
    #[error("Invalid CNB process name: {0}")]
    ProcessName(ProcessTypeError),
}

pub fn build_launch(pkgs: Vec<String>) -> Result<Launch, LaunchErr> {
    let mut launch = Launch::new();
    for pkg in pkgs {
        let proc = pkg
            .rsplit_once("/")
            .map(|(_path, name)| name)
            .ok_or_else(|| LaunchErr::PackageName(pkg.to_string()))?
            .parse::<ProcessType>()
            .map_err(LaunchErr::ProcessName)?;
        launch = launch.process(ProcessBuilder::new(proc.clone(), proc.to_string()).build());
    }
    Ok(launch)
}
