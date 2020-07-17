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

fn main() {
    // Bootstrapping
    pretty_env_logger::init();
}
