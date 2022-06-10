use libcnb::Env;
use std::process::{Command, ExitStatus};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GoCmdError {
    #[error("{0}")]
    IO(std::io::Error),
    #[error("{0}")]
    Exit(ExitStatus),
}

pub fn go_build(packages: Vec<&str>, target_dir: &str, go_env: &Env) -> Result<(), GoCmdError> {
    let mut args = vec!["build", "-o", target_dir];
    args.extend(packages);
    let mut build_cmd = Command::new("go")
        .args(args)
        .envs(go_env)
        .spawn()
        .map_err(GoCmdError::IO)?;

    let status = build_cmd.wait().map_err(GoCmdError::IO)?;

    status.success().then(|| ()).ok_or(GoCmdError::Exit(status))
}
