// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use libcnb_test::{assert_contains, assert_not_contains, BuildConfig, ContainerConfig, TestRunner};
use std::time::Duration;

const DEFAULT_BUILDER: &str = "heroku/builder:24";

fn test_go_fixture(fixture: &str, expect_loglines: &[&str], refute_loglines: &[&str]) {
    let builder = std::env::var("INTEGRATION_TEST_BUILDER").unwrap_or(DEFAULT_BUILDER.to_string());
    // TODO: Once Pack build supports `--platform` and libcnb-test adjusted accordingly, change this
    // to allow configuring the target arch independently of the builder name (eg via env var).
    let target_triple = match builder.as_str() {
        // Compile the buildpack for ARM64 iff the builder supports multi-arch and the host is ARM64.
        "heroku/builder:24" if cfg!(target_arch = "aarch64") => "aarch64-unknown-linux-musl",
        _ => "x86_64-unknown-linux-musl",
    };

    let mut build_config = BuildConfig::new(builder, format!("tests/fixtures/{fixture}"));
    build_config.target_triple(target_triple);
    TestRunner::default().build(build_config, |ctx| {
        let logs = format!("{}\n{}", ctx.pack_stdout, ctx.pack_stderr);
        for expect_line in expect_loglines {
            assert_contains!(logs, expect_line);
        }
        for refute_line in refute_loglines {
            assert_not_contains!(logs, refute_line);
        }

        let port = 8080;
        ctx.start_container(ContainerConfig::new().expose_port(port), |container| {
            std::thread::sleep(Duration::from_secs(5));
            let addr = container.address_for_port(port);
            let resp = ureq::get(&format!("http://{addr}"))
                .call()
                .expect("request to container failed")
                .into_string()
                .expect("response read error");

            assert_contains!(resp, fixture);
        });
    });
}

#[test]
#[ignore = "integration test"]
fn test_basic_http_116() {
    test_go_fixture(
        "basic_http_116",
        &[
            "Detected Go version requirement: ~1.16.2",
            "Resolved Go version: go1.16.",
            "Installing go1.16.",
        ],
        &[],
    );
}

#[test]
#[ignore = "integration test"]
fn test_vendor_gorilla_117() {
    test_go_fixture(
        "vendor_gorilla_117",
        &[
            "Detected Go version requirement: =1.17.8",
            "Installing go1.17.8",
            "Using vendored Go modules",
        ],
        &["downloading github.com/gorilla/mux v1.8.0"],
    );
}

#[test]
#[ignore = "integration test"]
fn test_modules_gin_121() {
    test_go_fixture(
        "modules_gin_121",
        &[
            "Detected Go version requirement: =1.21",
            "Installing go1.21",
            "downloading github.com/gin-gonic/gin v1.8.1",
        ],
        &[],
    );
}

#[test]
#[ignore = "integration test"]
fn test_worker_http_118() {
    test_go_fixture(
        "worker_http_118",
        &[
            "Detected Go version requirement: ~1.18.1",
            "Installing go1.18.",
            "example.com/worker_http_118/cmd/web",
            "example.com/worker_http_118/cmd/worker",
        ],
        &["example.com/worker_http_118/cmd/script"],
    );
}

#[test]
#[ignore = "integration test"]
fn test_basic_http_119() {
    test_go_fixture(
        "basic_http_119",
        &[
            "Detected Go version requirement: ~1.19.4",
            "Installing go1.19.",
        ],
        &[],
    );
}

#[test]
#[ignore = "integration test"]
fn test_vendor_fasthttp_120() {
    test_go_fixture(
        "vendor_fasthttp_120",
        &[
            "Detected Go version requirement: =1.20",
            "Installing go1.20.",
            "Using vendored Go modules",
        ],
        &["downloading github.com/valyala/fasthttp"],
    );
}

#[test]
#[ignore = "integration test"]
fn test_basic_http_122() {
    test_go_fixture(
        "basic_http_122",
        &[
            "Detected Go version requirement: ~1.22.0",
            "Installing go1.22.",
        ],
        &[],
    );
}

#[test]
#[ignore = "integration test"]
fn test_go_artifact_caching() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/basic_http_116"),
        |ctx| {
            assert_contains!(ctx.pack_stdout, "Installing go1.16.",);
            let config = ctx.config.clone();
            ctx.rebuild(config, |ctx| {
                assert_contains!(ctx.pack_stdout, "Reusing go1.16.");
            });
        },
    );
}
