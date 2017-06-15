#![feature(slice_patterns)]

extern crate hyper;
extern crate futures;

extern crate serde;
// #[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

mod core;
mod http;

use std::net::SocketAddr;
use std::sync::mpsc::channel;
use std::sync::mpsc::RecvTimeoutError;
use std::result::Result;
use std::thread;
use std::thread::JoinHandle;

use std::time::Duration;

use hyper::server::Http;

use futures::Future;
use futures::sync::oneshot;
use futures::sync::mpsc;
use futures::stream::Stream;
use futures::sink::Sink;

#[derive(Debug)]
pub enum ServerError {
    Hyper(hyper::Error),
    Assertion(&'static str)
}

impl From<hyper::Error> for ServerError {
    fn from(e: hyper::Error) -> ServerError {
        ServerError::Hyper(e)
    }
}

pub struct Server {
    local_addr: SocketAddr,
    shutdown_promise: mpsc::Sender<()>,
    join_handle: JoinHandle<Result<(), ServerError>>
}

impl Server {
    pub fn start(addr: SocketAddr) -> Result<Server, ServerError> {

        let (shutdown_sender, shutdown_receiver) = mpsc::channel::<()>(1);

        let shutdown_promise = shutdown_sender;
        let shutdown_future = shutdown_receiver.into_future();

        let (startup_promise, startup_future) = oneshot::channel::<SocketAddr>();

        let shutdown_promise_arg = shutdown_promise.clone();
        let child = thread::spawn(move || Server::run(addr, shutdown_promise_arg, shutdown_future, startup_promise));

        let actual_addr = match startup_future.wait() {
            Ok(_addr) => _addr,
            Err(_) => {
                match Self::_shutdown(shutdown_promise.clone(), child) {
                    Ok(_) => {
                        return Err(ServerError::Assertion(
                            "Error fetching local address, but server did not return error"
                        ))
                    }
                    Err(e) => return Err(e), // return the actual server error
                }
            }
        };

        // info!("Listening on http://{}...", actual_addr);

        Ok(Server {
            local_addr: actual_addr,
            shutdown_promise: shutdown_promise,
            join_handle: child
        })
    }

    pub fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }

    pub fn shutdown(self) -> Result<(), ServerError> {
        Self::_shutdown(self.shutdown_promise, self.join_handle)
    }

    pub fn join_timeout(self, timeout: Duration) -> Result<(), ServerError> {
        let (sender, receiver) = channel::<()>();

        // only way I know how - spin up another thread to wait for the first ...
        let waiter = thread::spawn(move || {
            Self::_join(self.join_handle).expect("Error joining server thread");
            sender.send(()).expect("Error sending success message on channel");
        });

        match receiver.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => Err(ServerError::Assertion("Timeout waiting for server thread to stop")),
            Err(_) => Err(ServerError::Assertion("Error joining server thread (channel disconnected)")),
            Ok(_) => {
                 // wait for 'waiter' thread to end
                match waiter.join() {
                    Err(_) => Err(ServerError::Assertion("Could not join water thread")),
                    Ok(_) => Ok(())
                }
            }
        }
    }

    pub fn join(self) -> Result<(), ServerError> {
        Self::_join(self.join_handle)
    }

    // fn _shutdown(shutdown_promise: oneshot::Sender<()>,
    fn _shutdown(shutdown_promise: mpsc::Sender<()>,
                 join_handle: JoinHandle<Result<(), ServerError>>)
                 -> Result<(), ServerError> {
        // if shutdown_promise.send(()).is_err() {
        //     warn!("shutdown(): Thread appears to be already ended");
        // }

        if shutdown_promise.send(()).wait().is_err() {
            warn!("shutdown(): Error queueing shutdown message"); // other end may have stopped listening
        }

        Self::_join(join_handle)
    }

    fn _join(join_handle: JoinHandle<Result<(), ServerError>>) 
                 -> Result<(), ServerError>{
        match join_handle.join() {
            Err(_) => Err(ServerError::Assertion("Could not join server thread")),
            Ok(r) => r,
        }
    }

    fn run<F, I, E>(addr: SocketAddr,
        //    shutdown_future: oneshot::Receiver<()>,
           shutdown_promise: mpsc::Sender<()>,
           shutdown_future: F, 
           startup_promise: oneshot::Sender<SocketAddr>) 
           -> Result<(), ServerError> 
           where F: Future<Item = I, Error = E> {

        // fn _strip<T, U>(t: T) -> U 
        //     where T: Future<Item = String, Error = String>, 
        //           U: Future<Item = (), Error = ()> {
        //     t.map_err(|_| ()).map(|_| ())
        // }

        let server = Http::new().bind(&addr, move || Ok(http::service::HttpService { 
            shutdown_promise: shutdown_promise.clone()
        }));

        // if server.is_err() {
        //     error!("Could not bind to address: {}", addr); // TODO: print IO error description here
        //     startup_promise.send(BindResult::BindError);
        //     return Err(server.map(|_| ()));
        // }

        let server = server.unwrap();

        // return actual listening address to parent thread
        startup_promise.send(server.local_addr()?).map_err(|_| {
            ServerError::Assertion("Could not return address to parent thread")
        })?;

        server.run_until(shutdown_future.map_err(|_| ()).map(|_| ()))?;
        // server.run_until(_strip(shutdown_future))?;

        Ok(()) // clean shutdown
    }
}
