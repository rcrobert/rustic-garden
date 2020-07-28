extern crate log;
extern crate pretty_env_logger;
extern crate sysfs_gpio;

#[macro_use]
extern crate static_assertions;

mod constants;
mod logbook;
mod valve;
mod calendar;
mod config_persist;
mod environment;

use tokio::prelude::*;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::mpsc;

use environment::{Environment, Service, ServiceKit, Any, AsAny};
use std::sync::{Arc, Weak};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bootstrapping
    pretty_env_logger::init();

    // Create our environment
    let (env_owned, env) = Environment::bootstrap();
    let env = unsafe { &mut *env };

    // Special shutdown handling
    let (sender, mut shutdown_receiver) = mpsc::channel::<bool>(100);

    // Start root services
    let kit = ServiceKit::with_env(env_owned, env)
        .with_dep::<RequestHandler>()
        .with_dep::<Taskmaster>()
        .new();

    env.finish_bootstrap();

    shutdown_receiver.recv().await;

    Ok(())
}

use tokio::net::{TcpListener, TcpStream};

// Services UI inbound requests
struct RequestHandler {
    kit: ServiceKit,
}

impl RequestHandler {
    async fn handle_request(socket: TcpStream) {
        // Do some stuff, respond to ui, use channels etc
    }
}

impl Service for RequestHandler {
    fn start(env_owned: Arc<Environment>, env: &mut Environment) -> RequestHandler {
        let inst = RequestHandler {
            kit: ServiceKit::with_env(env_owned, env).new(),
        };

        tokio::spawn(async move {
            let mut listener = TcpListener::bind("127.0.0.1:7292").await.expect("bind address succeeds");

            loop {
                let (socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    RequestHandler::handle_request(socket).await;
                });
            }
        });
        
        return inst;
    }

    fn name() -> &'static str {
        "RequestHandler"
    }
}

impl AsAny for RequestHandler {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Background checking of any needed schedule fulfilments
struct Taskmaster {}
impl Taskmaster {}

impl Service for Taskmaster {
    fn start(env_owned: Arc<Environment>, env: &mut Environment) -> Taskmaster {
        tokio::spawn(async move {
            // Poll some file state about schedules
            // Launch schedules
            // Compute wakeup
        });

        Taskmaster {}
    }

    fn name() -> &'static str {
        "Taskmaster"
    }
}

impl AsAny for Taskmaster {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
