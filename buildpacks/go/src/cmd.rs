use bullet_stream::{global::print, state::SubBullet, style, Print};
use fun_run::{CmdError, CommandWithName};
use libcnb::Env;
use std::{io::Write, process::Command};

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    FailedCommand(#[from] CmdError),

    #[error("Command IO error: {0}")]
    IO(#[from] std::io::Error),
}

/// Run `go install -tags heroku pkg [..pkgn]`. Useful for compiling a list
/// of packages and installing each of them in `GOBIN`. This command is module
/// aware, and will download required modules as a side-effect.
///
/// # Errors
///
/// Returns an error if the command exit code is not 0 or if there is an IO
/// issue with the command.
pub(crate) fn go_install<S: AsRef<str>, W: Write + Send + Sync + 'static>(
    mut bullet: Print<SubBullet<W>>,
    packages: &[S],
    go_env: &Env,
) -> Result<Print<SubBullet<W>>, Error> {
    let mut args = vec!["install", "-tags", "heroku"];
    for pkg in packages {
        args.push(pkg.as_ref());
    }
    let mut cmd = Command::new("go");
    cmd.args(args).envs(go_env);

    bullet
        .stream_with(
            format!("Running {}", style::command(cmd.name())),
            |stdout, stderr| cmd.stream_output(stdout, stderr),
        )
        .map(|_| bullet)
        .map_err(Error::FailedCommand)
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
    let mut cmd = Command::new("go");
    cmd.args(vec![
        "list",
        "-tags",
        "heroku",
        "-f",
        "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
        "./...",
    ])
    .envs(go_env);

    print::sub_stream_with(
        format!("Running {}", style::command(cmd.name())),
        |stdout, stderr| cmd.stream_output(stdout, stderr),
    )
    .map_err(Error::FailedCommand)
    .map(|output| {
        output
            .stdout_lossy()
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .collect()
    })
}
