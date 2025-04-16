use bullet_stream::global::print;
use fun_run::CmdError;
use libcnb::Env;
use std::process::{Command, ExitStatus};

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("Command IO error: {0}")]
    IO(#[from] std::io::Error),
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
pub(crate) fn go_install<S: AsRef<str>>(packages: &[S], go_env: &Env) -> Result<(), Error> {
    let mut args = vec!["install", "-tags", "heroku"];
    for pkg in packages {
        args.push(pkg.as_ref());
    }

    print::sub_stream_cmd(Command::new("go").args(args).envs(go_env)).map_err(command_error)?;
    Ok(())
}

fn command_error(error: CmdError) -> Error {
    if let CmdError::SystemError(_, error) = error {
        Error::IO(error)
    } else {
        Error::Exit(error.status())
    }
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
    let output = print::sub_stream_cmd(
        Command::new("go")
            .args(vec![
                "list",
                "-tags",
                "heroku",
                "-f",
                "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
                "./...",
            ])
            .envs(go_env),
    )
    .map_err(command_error)?;

    Ok(output
        .stdout_lossy()
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .collect())
}
