extern crate log;
extern crate pretty_env_logger;
extern crate sysfs_gpio;

pub mod constants;
pub mod logbook;
pub mod valve;
pub mod environment;
pub mod calendar;

use environment::Environment;

fn main() {
    // Bootstrapping
    pretty_env_logger::init();

    // This owns all service instances
    let env = Environment::new();
}
