use libcnb::data::{
    launch::{Launch, ProcessBuilder, ProcessType, ProcessTypeError},
    process_type,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchErr {
    #[error("Invalid Go package import path: {0}")]
    ImportPath(String),
    #[error("Invalid CNB process name: {0}")]
    ProcessName(ProcessTypeError),
}

pub fn build_launch(pkgs: Vec<String>) -> Result<Launch, LaunchErr> {
    let mut launch = Launch::new();
    for pkg in &pkgs {
        let proc = pkg
            .rsplit_once('/')
            .map(|(_path, name)| name)
            .ok_or_else(|| LaunchErr::ImportPath(pkg.to_string()))?
            .parse::<ProcessType>()
            .map_err(LaunchErr::ProcessName)?;
        launch = launch.process(
            ProcessBuilder::new(proc.clone(), proc.to_string())
                .default(proc.to_string() == "web")
                .build(),
        );
    }
    if !launch.processes.iter().any(|p| p.default) {
        if let Some(proc) = launch.processes.clone().get(0) {
            launch = launch.process(
                ProcessBuilder::new(process_type!("web"), &proc.command)
                    .default(true)
                    .build(),
            );
        }
    }
    Ok(launch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_launch_adds_web() {
        let launch = build_launch(vec![String::from("github.com/kubernetes/kubernetes")])
            .expect("unexpected error with build_launch");
        for (i, name) in ["kubernetes", "web"].iter().enumerate() {
            let proc = launch
                .processes
                .get(i)
                .expect("missing process in build_launch");
            assert_eq!(*name, proc.r#type.to_string());
            assert_eq!("kubernetes", proc.command);
        }
    }

    #[test]
    fn build_launch_does_not_dup_web() {
        let launch = build_launch(vec![String::from("example.com/web")])
            .expect("unexpected error with build_launch");
        assert_eq!(launch.processes.len(), 1);
        assert_eq!(launch.processes[0].command, "web");
    }

    #[test]
    fn build_launch_invalid_pkg() {
        let err = build_launch(vec![String::from("foobar")]).unwrap_err();
        assert_eq!(format!("{err}"), "Invalid Go package import path: foobar")
    }

    #[test]
    fn build_launch_invalid_process() {
        let err = build_launch(vec![String::from("example.com/[]")]).unwrap_err();
        assert_eq!(
            format!("{err}"),
            "Invalid CNB process name: Invalid Value: []"
        )
    }
}
