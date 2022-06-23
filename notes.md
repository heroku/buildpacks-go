### TODO:

- [x] Determine packages to install by looking for `main` packages with `go list
  -f`.
- [] Determine packages to install with the +heroku install build directive if
  present.
- [x] Install modules into the global module cache if they aren't vendored, and ensure the cache data is persisted and restored between builds.
- Verify vendored modules against go.sum, if it's present.
- [x] Cache the incremental build cache to speed up successive builds
- [x] Write launch.toml based on installed packages
  - [] If there is a web or server binary, set it as default and web
  - [x] If there is only one binary that is built and installed, it can be set as
    default and web.
  - [x] Otherwise set the alphabetically first binary as default
- [x] Validate go distribution sha on installation.
- [] Use `-tags heroku` during `go install`
- [] Git credential helper for private dependencies. Maybe this should be another
  buildpack? Or a netrc buildpack?

### Layers:

#### `go_dist`

`go` itself is installed here. This layer is available during the build and is
cached. It is not available at runtime.

#### `go_deps`

go dependencies are installed here, if they aren't vendored in the app.
This layer is available during the build and is cached. It is not available at runtime.

#### `go_build`

the go build cache is stored here to enable incremental builds.

#### `go_target`

compiled app binaries are installed here. This wil be needed at runtime, but not
during build, and will not be cached.
