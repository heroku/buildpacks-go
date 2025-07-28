## Application Specification

The sections below describe the expected behavior of the buildpack. The codebase must be updated if a difference exists between this contract and actual behavior. Either the code needs to change to suit the specification, or the specification needs to be updated. If you see a difference, please open an issue with a [minimal application that reproduces the problem](https://www.codetriage.com/example_app).

If you need application-specific support, you can ask on an official Heroku support channel or Stack Overflow.

### Application Specification: Detect

The detect phase determines whether this buildpack can execute. It can also be used to request additional functionality by requiring behavior from other buildpacks.

- Given a `go.mod` file in the root directory of the application, this buildpack will execute the build contract specified below.
- If no `go.mod` file is found, another buildpack can request a default version of Go to be installed by requiring `go` in their build plan. When that happens, this buildpack will execute the build contract specified below.
  - A requiring buildpack must:
    - Write a valid `go.mod` file to disk before this buildpack executes.
  - To aid in debugging, a requiring buildpack SHOULD print a message to the user that states:
    - When the `go.mod` file is generated.
    - The contents of the written `go.mod`.
    - The source of any information used in generating the `go.mod` file.

### Application Specification: Build

Once an application has passed the detect phase, the build phase will execute to prepare the application to run.

- Go version
  - An application must have a `go.mod` file. If there is no `go.mod` file in the root directory, this buildpack will error.
  - Go version is resolved using SemVer rules and symbols documented in the [semver crate](https://docs.rs/semver/latest/semver/enum.Op.html)'s documentation. (Check `Cargo.lock` for exact version).
  - An empty `go.mod` file, or a file absent any Go version information, will resolve to the semver specifier `*` and install the latest available Go version.
  - A `go.mod` file with a "go line" will be used when no "heroku go version comment" is provided. A "go line" follows the structure: `go<space><constraint><version>`. For example, `go 1.17` resolves to requirement `=1.17`
    - `<space>` must be whitespace or empty
    - `<constraint>` must be one of: `=`, `>`, `>=`, `<`, `<=`, `~`, `^`, `*` or empty.
      - When the constraint is empty, it will be treated as `=`
    - `<version>` must be numbers and periods only and cannot be empty.
  - A `go.mod` file with a "heroku go version comment" will resolve as a semver version and not use the "go version line" if one is present. The SemVer identifier will be used to determine the installed Go version according to the SemVer rules.
    - A "heroku go version comment" follows the structure: `// +heroku goVersion <constraint><version>`. For example: `// +heroku goVersion =1.18.4` resolves to requirement `=1.18.4`
      - `<constraint>` must be one of: `=`, `>`, `>=`, `<`, `<=`, `~`, `^`, `*` or empty.
      - When the constraint is empty, it will be treated as `=`
      - `<version>` must be numbers and periods only and cannot be empty.
  - A distribution of `go` and `gofmt` are available on the `PATH` for other buildpacks based on the above go version resolution, but not at runtime.
- Package detection
  - Packages for installation are determined via `go.mod` contents
  - A `go.mod` file with a "heroku package comment" will build the specified packages.
    - A "heroku package comment" follows the structure: `// +heroku install <pkgspec>`. For example, this would install `example.com/example-server` and `example.com/example-worker`:

        // +heroku install example.com/example-server example.com/example-worker

      - `<pkgspec>` is one or more space delimited packages.
  - Given no explicit packages are specified via a "heroku package comment" the buildpack will run `go list -tags heroku` and build all packages that have a name of `main`.

- Package installation
  - This buildpack will execute `go install -tags heroku <pkgspec>` where `<pkgspec>` is one or more packages determined above. This command installs dependencies in addition to building the requested packages.
- Package registration
  - Given no `Procfile` exists at the application root, this buildpack will convert go packages into a list of processes that can be executed.
    - Given a package has a `web` suffix like `example.com/example-web` it will be marked as a default process.
      - If no process has a `web` suffix, the first process returned above will be marked as a default process.
    - Package names must include a slash `/` and the process name will be taken from the name after the slash. So `example.com/example-web` would become a process type named `example-web`
    - Package names MUST comply with the upstream [buildpack spec](https://github.com/buildpacks/spec/blob/main/buildpack.md#launchtoml-toml) (only contain numbers, letters, and the characters `.`, `_`, and `-`).

- Environment variables modified
  - `GOBIN` - The location of installed application binaries (packages) compiled via `go install`.
  - `PATH` - The following entries are prepended to the `PATH`:
    - `$GOBIN` (build and runtime)
    - `$GOROOT/bin` (build only)
  - `GOROOT` (build only) - The GOROOT directory for the `go` command that invoked the generator, containing the Go toolchain and standard library.
  - `GOMODCACHE` (build only) - This environment variable points at a cache directory used for go dependency installation. May not be present if using vendored modules, detected by `<app-dir>/vendor/modules.txt` presence.
  - `GOCACHE` (build only) - The go build cache.
  - `GO111MODULE=on` (build only) - Runs `go` commands in "module aware" mode. See https://golang.org/ref/mod#mod-commands.
