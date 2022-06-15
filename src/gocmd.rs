use libcnb::Env;
use std::process::{Command, ExitStatus, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GoCmdError {
    #[error("{0}")]
    IO(std::io::Error),
    #[error("{0}")]
    Exit(ExitStatus),
}

pub fn go_install<S: AsRef<str>>(packages: &[S], go_env: &Env) -> Result<(), GoCmdError> {
    let mut args = vec!["install"];
    for pkg in packages {
        args.push(pkg.as_ref());
    }
    let mut build_cmd = Command::new("go")
        .args(args)
        .envs(go_env)
        .spawn()
        .map_err(GoCmdError::IO)?;

    let status = build_cmd.wait().map_err(GoCmdError::IO)?;

    status.success().then(|| ()).ok_or(GoCmdError::Exit(status))
}

pub fn go_list(go_env: &Env) -> Result<Vec<String>, GoCmdError> {
    let list_cmd = Command::new("go")
        .args(vec![
            "list",
            "-f",
            "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
            "./...",
        ])
        .envs(go_env)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(GoCmdError::IO)?;

    let result = list_cmd.wait_with_output().map_err(GoCmdError::IO)?;

    result
        .status
        .success()
        .then(|| ())
        .ok_or(GoCmdError::Exit(result.status))?;

    Ok(String::from_utf8_lossy(&result.stdout)
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .collect())
}
