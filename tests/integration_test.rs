#![warn(clippy::pedantic)]

use libcnb_test::{assert_contains, BuildpackReference, IntegrationTest};
use std::time::Duration;

fn test_go_fixture(fixture: &str, expected_out: Vec<&str>, expected_err: Vec<&str>) {
    for stack in ["heroku/buildpacks:20", "heroku/builder:22"] {
        IntegrationTest::new(stack, format!("tests/fixtures/{fixture}"))
            .buildpacks(vec![BuildpackReference::Crate])
            .run_test(|ctx| {
                for out_phrase in expected_out.clone() {
                    assert_contains!(ctx.pack_stdout, out_phrase);
                }
                for err_phrase in expected_err.clone() {
                    assert_contains!(ctx.pack_stderr, err_phrase);
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
fn test_basic_http_118() {
    test_go_fixture(
        "basic_http_118",
        vec![
            "Detected Go version requirement: ~1.18",
            "Installing Go 1.18.",
        ],
        vec![],
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
        ],
        vec!["downloading github.com/gorilla/mux v1.8.0"],
    );
}
