// Required due to: https://github.com/rust-lang/rust/issues/95513
#![allow(unused_crate_dependencies)]

use libcnb_test::{assert_contains, assert_not_contains, BuildConfig, ContainerConfig, TestRunner};
use std::{env::consts, time::Duration};

const DEFAULT_BUILDER: &str = "heroku/builder:24";

struct IntegrationTestConfig {
    target: String,
    builder: String,
    fixture: String,
}

impl IntegrationTestConfig {
    fn new<S: Into<String>>(fixture: S) -> Self {
        let builder =
            std::env::var("INTEGRATION_TEST_BUILDER").unwrap_or(DEFAULT_BUILDER.to_string());
        let target = match (builder.as_str(), consts::ARCH) {
            // Compile the buildpack for arm64 if the builder supports multi-arch and the host is ARM64.
            // This happens in CI and on developer machines with Apple silicon.
            ("heroku/builder:24", "aarch64") => "aarch64-unknown-linux-musl".to_string(),
            // Compile the buildpack for arm64 if an arm64-specific builder is chosen.
            // Used to run cross-arch integration tests from machines with Intel silicon.
            (b, _) if b.ends_with("arm64") => "aarch64-unknown-linux-musl".to_string(),
            (_, _) => "x86_64-unknown-linux-musl".to_string(),
        };
        let fixture = format!("tests/fixtures/{}", fixture.into());
        Self {
            target,
            builder,
            fixture,
        }
    }
}

impl From<IntegrationTestConfig> for BuildConfig {
    fn from(integration_test_config: IntegrationTestConfig) -> BuildConfig {
        let mut build_config = BuildConfig::new(
            integration_test_config.builder,
            integration_test_config.fixture,
        );
        build_config.target_triple(integration_test_config.target);
        build_config
    }
}

fn test_go_fixture(fixture: &str, expect_loglines: &[&str], refute_loglines: &[&str]) {
    let build_config: BuildConfig = IntegrationTestConfig::new(fixture).into();
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
        &[&requirement_line("~1.16.2"), &installing_line("go1.16.")],
        &[],
    );
}

fn requirement_line(requirement: &str) -> String {
    format!("Detected requirement `{requirement}`")
}

fn installing_line(version: &str) -> String {
    format!("Installing `{version}")
}

#[test]
#[ignore = "integration test"]
fn test_vendor_gorilla_117() {
    test_go_fixture(
        "vendor_gorilla_117",
        &[
            &requirement_line("=1.17.8"),
            &installing_line("go1.17.8"),
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
            &requirement_line("=1.21"),
            &installing_line("go1.21"),
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
            &requirement_line("~1.18.1"),
            &installing_line("go1.18"),
            "Default processes",
            "- `example.com/worker_http_118/cmd/web`",
            "- `example.com/worker_http_118/cmd/worker`",
        ],
        &["- `example.com/worker_http_118/cmd/script`"],
    );
}

#[test]
#[ignore = "integration test"]
fn test_basic_http_119() {
    test_go_fixture(
        "basic_http_119",
        &[&requirement_line("~1.19.4"), &installing_line("go1.19")],
        &[],
    );
}

#[test]
#[ignore = "integration test"]
fn test_procfile_http_123() {
    let build_config: BuildConfig = IntegrationTestConfig::new("procfile_http_123").into();
    TestRunner::default().build(build_config, |ctx| {
        let logs = format!("{}\n{}", ctx.pack_stdout, ctx.pack_stderr);
        assert_contains!(logs, &requirement_line("=1.23"));
        assert_contains!(logs, &installing_line("go1.23."));
        assert_contains!(logs, "Skipping (Procfile detected)");
    });
}

#[test]
#[ignore = "integration test"]
fn test_vendor_fasthttp_120() {
    test_go_fixture(
        "vendor_fasthttp_120",
        &[
            &requirement_line("=1.20"),
            &installing_line("go1.20."),
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
        &[&requirement_line("~1.22.0"), &installing_line("go1.22.")],
        &[],
    );
}

#[test]
#[ignore = "integration test"]
fn test_go_artifact_caching() {
    let build_config: BuildConfig = IntegrationTestConfig::new("basic_http_116").into();
    TestRunner::default().build(build_config, |ctx| {
        let logs = format!("{}\n{}", ctx.pack_stdout, ctx.pack_stderr);
        assert_contains!(&logs, &installing_line("go1.16."));
        let config = ctx.config.clone();
        ctx.rebuild(config, |ctx| {
            let logs = format!("{}\n{}", ctx.pack_stdout, ctx.pack_stderr);
            assert_contains!(&logs, "Resolved to `go1.16.");
            assert_contains!(&logs, "Using cache");
        });
    });
}

#[test]
#[ignore = "integration test"]
fn test_go_binary_arch() {
    let integration_config = IntegrationTestConfig::new("basic_http_122");
    let (contains, not_contain) = match integration_config.target.as_str() {
        "aarch64-unknown-linux-musl" => (["(linux-arm64)", "linux-arm64.tar.gz"], "amd64"),
        _ => (["(linux-amd64)", "linux-amd64.tar.gz"], "arm64"),
    };

    let build_config: BuildConfig = integration_config.into();
    TestRunner::default().build(build_config, |ctx| {
        let logs = format!("{}\n{}", ctx.pack_stdout, ctx.pack_stderr);
        for contain in contains {
            assert_contains!(&logs, contain);
        }
        assert_not_contains!(&logs, not_contain);
    });
}
