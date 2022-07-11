use libcnb::Env;
use std::process::{Command, ExitStatus, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CmdError {
    #[error("Command IO error: {0}")]
    IO(std::io::Error),
    #[error("Command did not exit successfully: {0}")]
    Exit(ExitStatus),
}

/// Run `go install -tags heroku pkg [..pkgn]`. Useful for compiling a list
/// of packages and installing each of them in `GOBIN`. This command is module
/// aware, and will download required modules as a side-effect.
///
/// # Errors
///
/// Returns an error if the command exit code is not 0 or if there is an IO
/// issue with the command.
pub fn go_install<S: AsRef<str>>(packages: &[S], go_env: &Env) -> Result<(), CmdError> {
    let mut args = vec!["install", "-tags", "heroku"];
    for pkg in packages {
        args.push(pkg.as_ref());
    }
    let mut build_cmd = Command::new("go")
        .args(args)
        .envs(go_env)
        .spawn()
        .map_err(CmdError::IO)?;

    let status = build_cmd.wait().map_err(CmdError::IO)?;

    status.success().then(|| ()).ok_or(CmdError::Exit(status))
}

/// Run `go list -tags -f {{ .ImportPath }} ./...`. Useful for listing
/// `main` packages in a go project to deterimine which packages to build.
/// This command is module aware, and will download required modules as a
/// side-effect.
///
/// # Errors
///
/// Returns an error if the command exit code is not 0 or if there is an IO
/// issue with the command.
pub fn go_list(go_env: &Env) -> Result<Vec<String>, CmdError> {
    let list_cmd = Command::new("go")
        .args(vec![
            "list",
            "-tags",
            "heroku",
            "-f",
            "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
            "./...",
        ])
        .envs(go_env)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(CmdError::IO)?;

    let result = list_cmd.wait_with_output().map_err(CmdError::IO)?;

    result
        .status
        .success()
        .then(|| ())
        .ok_or(CmdError::Exit(result.status))?;

    Ok(String::from_utf8_lossy(&result.stdout)
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .collect())
}
