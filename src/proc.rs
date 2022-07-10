use libcnb::data::{
    launch::{Process, ProcessBuilder, ProcessType, ProcessTypeError},
    process_type,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcErr {
    #[error("Invalid Go package import path: {0}")]
    ImportPath(String),
    #[error("Invalid CNB process name: {0}")]
    ProcessName(ProcessTypeError),
}

/// Turns a list of go packages into a CNB process list. Any package with
/// a `web` suffix will be flagged as default process. If there are packages
/// and none with a `web` suffix, a `web` process will be created for the
/// first package.
///
/// # Examples
///
/// ```
/// let procs = heroku_go_buildpack::proc::build_procs(
///                &["github.com/heroku/maple".to_string()]
///              ).unwrap();
/// ```
///
/// # Errors
///
/// Invalid go packages (those without a `'/'`) and go packages with suffixes
/// that don't satisfy CNB process naming conventions will error.
pub fn build_procs(pkgs: &[String]) -> Result<Vec<Process>, ProcErr> {
    let mut procs: Vec<Process> = vec![];
    for pkg in pkgs {
        let proc_name = pkg
            .rsplit_once('/')
            .map(|(_path, name)| name)
            .ok_or_else(|| ProcErr::ImportPath(pkg.to_string()))?
            .parse::<ProcessType>()
            .map_err(ProcErr::ProcessName)?;

        procs.push(
            ProcessBuilder::new(proc_name.clone(), proc_name.to_string())
                .default(proc_name.to_string() == "web")
                .build(),
        );
    }
    if !procs.iter().any(|p| p.default) {
        if let Some(proc) = procs.clone().get(0) {
            procs.push(
                ProcessBuilder::new(process_type!("web"), &proc.command)
                    .default(true)
                    .build(),
            );
        }
    }
    Ok(procs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_procs_adds_web() {
        let procs = build_procs(&[String::from("github.com/kubernetes/kubernetes")])
            .expect("unexpected error with build_procs");
        for (i, name) in ["kubernetes", "web"].iter().enumerate() {
            let proc = procs.get(i).expect("missing process in build_procs");
            assert_eq!(*name, proc.r#type.to_string());
            assert_eq!("kubernetes", proc.command);
        }
    }

    #[test]
    fn build_procs_does_not_dup_web() {
        let procs = build_procs(&[String::from("example.com/web")])
            .expect("unexpected error with build_procs");
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].command, "web");
    }

    #[test]
    fn build_procs_invalid_pkg() {
        let err = build_procs(&[String::from("foobar")]).unwrap_err();
        assert_eq!(format!("{err}"), "Invalid Go package import path: foobar");
    }

    #[test]
    fn build_procs_invalid_process() {
        let err = build_procs(&[String::from("example.com/[]")]).unwrap_err();
        assert_eq!(
            format!("{err}"),
            "Invalid CNB process name: Invalid Value: []"
        );
    }
}
