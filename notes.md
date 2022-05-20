### TODO:

- Determine packages to install by looking for `main` packages with `go list
  -f`.
- Determine packages to install with the +heroku install build directive if
  present.
- Install modules into the global module cache if they aren't vendored, and ensure the cached is cached and restored between builds.
- Verify modules against go.sum, if it's present.
- Write launch.toml based on installed packages
  - If there is only package that is built and installed, it can be set as
    default and web.
- Validate go distribution sha on installation.
- Git credential helper for private dependencies. Maybe this should be another
  buildpack? Or a netrc buildpack?

### Layers:

#### `dist`

`go` itself is installed here. This layer is available during the build and is
cached. It is not available at runtime.

#### `deps`

go dependencies are installed here, if they aren't vendored in the app. 
This layer is available during the build and is cached. It is not available at runtime.

#### `out`

compiled app binaries are installed here. This wil be needed at runtime, but not during build.
This should probably not be cached.

