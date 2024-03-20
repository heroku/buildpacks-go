// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use libcnb_test::{assert_contains, assert_not_contains, BuildConfig, ContainerConfig, TestRunner};
use std::time::Duration;

fn test_go_fixture(
    fixture: &str,
    builder: &str,
    expect_loglines: &[&str],
    refute_loglines: &[&str],
) {
    TestRunner::default().build(
        BuildConfig::new(builder, format!("tests/fixtures/{fixture}")),
        |ctx| {
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
        },
    );
}

fn test_basic_http_116(builder: &str) {
    test_go_fixture(
        "basic_http_116",
        builder,
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
fn basic_http_116_20() {
    test_basic_http_116("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn basic_http_116_22() {
    test_basic_http_116("heroku/builder:22");
}

fn test_vendor_gorilla_117(builder: &str) {
    test_go_fixture(
        "vendor_gorilla_117",
        builder,
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
fn vendor_gorilla_117_20() {
    test_vendor_gorilla_117("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn vendor_gorilla_117_22() {
    test_vendor_gorilla_117("heroku/builder:22");
}

fn test_modules_gin_121(builder: &str) {
    test_go_fixture(
        "modules_gin_121",
        builder,
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
fn modules_gin_121_20() {
    test_modules_gin_121("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn modules_gin_121_22() {
    test_modules_gin_121("heroku/builder:22");
}

fn test_worker_http_118(builder: &str) {
    test_go_fixture(
        "worker_http_118",
        builder,
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
fn worker_http_118_20() {
    test_worker_http_118("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn worker_http_118_22() {
    test_worker_http_118("heroku/builder:22");
}

fn test_basic_http_119(builder: &str) {
    test_go_fixture(
        "basic_http_119",
        builder,
        &[
            "Detected Go version requirement: ~1.19.4",
            "Installing go1.19.",
        ],
        &[],
    );
}
#[test]
#[ignore = "integration test"]
fn basic_http_119_20() {
    test_basic_http_119("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn basic_http_119_22() {
    test_basic_http_119("heroku/builder:22");
}

fn test_vendor_fasthttp_120(builder: &str) {
    test_go_fixture(
        "vendor_fasthttp_120",
        builder,
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
fn vendor_fasthttp_120_20() {
    test_vendor_fasthttp_120("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn vendor_fasthttp_120_22() {
    test_vendor_fasthttp_120("heroku/builder:22");
}

fn test_basic_http_122(builder: &str) {
    test_go_fixture(
        "basic_http_122",
        builder,
        &[
            "Detected Go version requirement: ~1.22.0",
            "Installing go1.22.",
        ],
        &[],
    );
}
#[test]
#[ignore = "integration test"]
fn basic_http_122_20() {
    test_basic_http_122("heroku/builder:20");
}
#[test]
#[ignore = "integration test"]
fn basic_http_122_22() {
    test_basic_http_122("heroku/builder:22");
}

#[test]
#[ignore = "integration test"]
fn test_go_artifact_caching() {
    TestRunner::default().build(
        BuildConfig::new("heroku/builder:22", "tests/fixtures/basic_http_116"),
        |ctx| {
            assert_contains!(
                ctx.pack_stdout,
                "Installing go1.16.15 (linux-x86_64) from https://go.dev/dl/go1.16.15.linux-amd64.tar.gz",
            );
            let config = ctx.config.clone();
            ctx.rebuild(config, |ctx| {
                assert_contains!(ctx.pack_stdout, "Reusing go1.16.15 (linux-x86_64)");
            });
        },
    );
}
