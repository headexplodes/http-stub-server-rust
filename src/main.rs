extern crate hyper;
extern crate futures;

use hyper::StatusCode;
use hyper::server::{Http, Request, Response, Service};
use hyper::header::{ContentLength, ContentType};

static TEXT: &'static str = "Hello, World! Now, to do something more important ...";

struct HelloWorld;

impl Service for HelloWorld {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;

    // The future representing the eventual Response your call will
    // resolve to. This can change to whatever Future you need.
    type Future = futures::future::FutureResult<Self::Response, Self::Error>;

    fn call(&self, _req: Request) -> Self::Future {
        futures::future::ok(
            Response::new()
                .with_status(StatusCode::Ok)
                .with_header(ContentType::plaintext())
                .with_header(ContentLength(TEXT.len() as u64))
                .with_body(TEXT)
        )
    }
}

fn main() {
    let addr = "127.0.0.1:3000".parse().unwrap();
    let server = Http::new().bind(&addr, || Ok(HelloWorld)).unwrap();
    println!("Listening on http://{} with 1 thread.", server.local_addr().unwrap());
    server.run().unwrap();
}
