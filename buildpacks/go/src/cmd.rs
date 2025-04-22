use bullet_stream::global::print;
use fun_run::{CmdError, CommandWithName, NamedCommand};
use libcnb::Env;
use std::process::Command;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    Command(CmdError),
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

    print::sub_stream_cmd(Command::new("go").args(args).envs(go_env)).map_err(Error::Command)?;
    Ok(())
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
    let mut command = std::process::Command::new("go");
    let mut short: NamedCommand = command
        .envs(go_env)
        .args(["list", "-tags", "heroku"])
        .into();
    // Hide these (possibly confusing) flags from build output
    short.mut_cmd().args([
        "-f",
        "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
        "./...",
    ]);
    let output = print::sub_stream_cmd(short).map_err(Error::Command)?;

    Ok(output
        .stdout_lossy()
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .collect())
}
