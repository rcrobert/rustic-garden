extern crate log;
extern crate pretty_env_logger;
extern crate sysfs_gpio;

#[macro_use]
extern crate static_assertions;

mod constants;
mod logbook;
mod valve;
mod environment;
mod calendar;
mod config_persist;

use environment::Environment;

fn main() {
    // Bootstrapping
    pretty_env_logger::init();

    // This owns all service instances
    let env = Environment::new();
}
