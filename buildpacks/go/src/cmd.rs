use libcnb::Env;
use std::process::{Command, ExitStatus, Stdio};

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Command IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Command did not exit successfully: {0}")]
    Exit(ExitStatus),
}

/// Run `go clean -tags heroku`. Useful for clearing the modcache or buildcache.
///
/// # Errors
///
/// Returns an error of the command exit code is not 0 or if there is an IO
/// issue with the command.
pub(crate) fn go_clean<S: AsRef<str>>(flag: S, go_env: &Env) -> Result<(), Error> {
    let status = Command::new("go")
        .args(["clean", "-tags", "heroku", flag.as_ref()])
        .envs(go_env)
        .status()?;

    status.success().then_some(()).ok_or(Error::Exit(status))
}

/// Run `go install -tags heroku pkg [..pkgn]`. Useful for compiling a list
/// of packages and installing each of them in `GOBIN`. This command is module
/// aware, and will download required modules as a side-effect.
///
/// # Errors
///
/// Returns an error if the command exit code is not 0 or if there is an IO
/// issue with the command.
pub(crate) fn go_install<S: AsRef<str>>(packages: &[S], go_env: &Env) -> Result<(), Error> {
    let mut args = vec!["install", "-tags", "heroku"];
    for pkg in packages {
        args.push(pkg.as_ref());
    }
    let status = Command::new("go").args(args).envs(go_env).status()?;
    status.success().then_some(()).ok_or(Error::Exit(status))
}

/// Run `go list -tags -f {{ .ImportPath }} ./...`. Useful for listing
/// `main` packages in a go project to determine which packages to build.
/// This command is module aware, and will download required modules as a
/// side-effect.
///
/// # Errors
///
/// Returns an error if the command exit code is not 0 or if there is an IO
/// issue with the command.
pub(crate) fn go_list(go_env: &Env) -> Result<Vec<String>, Error> {
    let result = Command::new("go")
        .args(vec![
            "list",
            "-tags",
            "heroku",
            "-f",
            "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
            "./...",
        ])
        .envs(go_env)
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .output()?;

    result
        .status
        .success()
        .then_some(())
        .ok_or(Error::Exit(result.status))?;

    Ok(String::from_utf8_lossy(&result.stdout)
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .collect())
}
