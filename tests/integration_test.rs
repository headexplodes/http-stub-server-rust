extern crate stubby;

#[macro_use]
extern crate log;
extern crate log4rs;

#[macro_use]
extern crate lazy_static;

extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde_json;
extern crate regex;

use std::io;
use std::io::Write;
use std::time::Duration;

use futures::Future;
use futures::Stream;

use hyper::header::{ContentLength, ContentType};
use hyper::{Chunk, Client, StatusCode, Method, Request};

use tokio_core::reactor::Core;

use serde_json::Value;

use stubby::Server;

use regex::Regex;

const LISTEN_ADDR: &'static str = "127.0.0.1:0"; // choose a free port for each test

fn start_server() -> Server {
    let addr = LISTEN_ADDR.parse().unwrap();
    
    let server = Server::start(addr).expect("Server started");
    
    info!("Server started on http://{}.", server.local_addr());

    return server;
}

lazy_static! {
    static ref LOG_INIT: Result<(), log4rs::Error> =
            log4rs::init_file("log4rs.yml", Default::default());
}

fn before() {
    // log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    // LOG_INIT.is_none(); // make sure logging initialised

    let l: &Result<(), log4rs::Error> = &LOG_INIT;
    assert!(l.is_ok());
}

// TODO: always run with '--test-threads'

// TODO: https://medium.com/@ericdreichert/test-setup-and-teardown-in-rust-without-a-framework-ba32d97aa5ab

// TODO: https://klausi.github.io/rustnish/2017/05/25/writing-integration-tests-in-rust.html

#[test]
fn test_version() {
    before();

    let server = start_server();

    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());

    let uri = format!("http://{}/_control/version", server.local_addr()).parse().unwrap();

    info!("Using URI: {}", uri);

    let work = client.get(uri).and_then(|res| {

        assert_eq!(res.status(), StatusCode::Ok);

        // res.body().for_each(|chunk| {
        //     io::stdout()
        //         .write_all(&chunk)
        //         .map(|_| ())
        //         .map_err(From::from)
        // })

        res.body().concat2().and_then(move |body: Chunk| {
            let json: Value = serde_json::from_slice(&body).unwrap();

            assert!(json.is_object());
            assert!(json.get("version").is_some());

            let version_re = Regex::new(r"^\d+.\d+.\d+$").unwrap();
            assert!(version_re.is_match(json["version"].as_str().unwrap()));

            Ok(())
        })

    });

    core.run(work).unwrap();

    server.shutdown().expect("Clean server shutdown");

    info!("Server shutdown.");
}

#[test]
fn test_shutdown() {
    before();

    let server = start_server();

    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());

    let uri = format!("http://{}/_control/shutdown", server.local_addr()).parse().unwrap();

    info!("Using URI: {}", uri);

    let req = Request::new(Method::Post, uri);
    // req.headers_mut().set(ContentType::json());
    // req.headers_mut().set(ContentLength(0));
    // req.set_body(json);

    let work = client.request(req).and_then(|res| {

        assert_eq!(res.status(), StatusCode::Accepted);

        res.body().concat2().and_then(move |body: Chunk| {
            let json: Value = serde_json::from_slice(&body).unwrap();

            assert!(json.is_object());
            assert!(json.get("message").is_some());
            assert_eq!(json["message"].as_str(), Some("Shutdown triggered"));

            Ok(())
        })

    });

    core.run(work).unwrap();

    // expect server to shutdown pretty quickly
    server.join_timeout(Duration::from_secs(5)).expect("Clean shutdown");

    info!("Server shutdown.");
}