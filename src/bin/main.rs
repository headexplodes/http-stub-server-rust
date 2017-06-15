extern crate stubby;

#[macro_use]
extern crate log;
extern crate log4rs;

use stubby::Server;

// TODO: clippy https://github.com/Manishearth/rust-clippy

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    info!("Starting up...");
 
    let addr = "127.0.0.1:3000".parse().unwrap();

    let server = match Server::start(addr) {
        Ok(s) => s,
        Err(e) => {
            error!("Error starting server: {:?}", e);
            return;
        }
    };
    
    info!("Listening on http://{}...", server.local_addr());

    match server.join() {
        Ok(_) => {
            info!("Server finished");
        },
        Err(e) => {
            error!("Server ended with error: {:?}", e);
            return;
        }
    }
}
