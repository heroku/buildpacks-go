### TODO:

- Determine packages to install by looking for `main` packages with `go list
  -f`.
- Determine packages to install with the +heroku install build directive if
  present.
- Install modules into the global module cache if they aren't vendored, and ensure the cache data is persisted and restored between builds.
- Verify modules against go.sum, if it's present.
- Write launch.toml based on installed packages
  - If there is only package that is built and installed, it can be set as
    default and web.
- Validate go distribution sha on installation.
- There's a cross-device issue when renaming from /tmp/go/bin/go to
  /layers/bin/go, so for now we're copying the file instead. Maybe if we 
  extract the tarball to a cache=false, build=false, run=false (which serves 
  as a tmp directory of sorts), we can rename the go binary instead of copying it.
- Git credential helper for private dependencies. Maybe this should be another
  buildpack? Or a netrc buildpack?

### Layers:

#### `dist`

`go` itself is installed here. This layer is available during the build and is
cached. It is not available at runtime.

#### `deps`

go dependencies are installed here, if they aren't vendored in the app. 
This layer is available during the build and is cached. It is not available at runtime.

#### `build`

the go build cache is stored here to enable incremental builds. 

#### `target`

compiled app binaries are installed here. This wil be needed at runtime, but not
during build, and will not be cached.
