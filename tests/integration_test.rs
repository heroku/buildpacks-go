#![warn(clippy::pedantic)]

use libcnb_test::{assert_contains, BuildpackReference, IntegrationTest};
use std::time::Duration;

fn test_go_fixture(fixture: &str, expected_loglines: Vec<&str>) {
    for stack in ["heroku/buildpacks:20", "heroku/builder:22"] {
        IntegrationTest::new(stack, format!("tests/fixtures/{fixture}"))
            .buildpacks(vec![BuildpackReference::Crate])
            .run_test(|ctx| {
                let logs = format!("{}\n{}", ctx.pack_stdout, ctx.pack_stderr);
                for logline in &expected_loglines {
                    assert_contains!(logs, logline);
                }
                let port = 8080;
                ctx.prepare_container()
                    .expose_port(port)
                    .start_with_default_process(|container| {
                        std::thread::sleep(Duration::from_secs(5));
                        let addr = container
                            .address_for_port(port)
                            .expect("couldn't get container address");
                        let resp = ureq::get(&format!("http://{addr}"))
                            .call()
                            .expect("request to container failed")
                            .into_string()
                            .expect("response read error");

                        assert_contains!(resp, fixture);
                    });
            });
    }
}

#[test]
#[ignore]
fn test_basic_http_116() {
    test_go_fixture(
        "basic_http_116",
        vec![
            "Detected Go version requirement: ~1.16.2",
            "Installing Go 1.16.",
        ],
    );
}

#[test]
#[ignore]
fn test_modules_gorilla_117() {
    test_go_fixture(
        "modules_gorilla_117",
        vec![
            "Detected Go version requirement: = 1.17.8",
            "Installing Go 1.17.8",
            "downloading github.com/gorilla/mux v1.8.0",
        ],
    );
}

#[test]
#[ignore]
fn test_vendor_gin_118() {
    test_go_fixture(
        "vendor_gin_118",
        vec![
            "Detected Go version requirement: = 1.18",
            "Installing Go 1.18",
        ],
    );
}

#[test]
#[ignore]
fn test_worker_http_118() {
    test_go_fixture(
        "worker_http_118",
        vec![
            "Detected Go version requirement: ~1.18.",
            "Installing Go 1.18.",
            "example.com/worker_http_118/cmd/web",
            "example.com/worker_http_118/cmd/worker",
        ],
    )
}
