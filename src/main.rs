extern crate sysfs_gpio;

use signal_hook;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use sysfs_gpio::{Direction, Pin};

fn main() {
    let (sender, _) = channel::<i32>();

    // Listen to signals
    thread::spawn(move || {
        let signals = signal_hook::iterator::Signals::new(&[signal_hook::SIGINT])
            .expect("Could not subscribe to signals");
        for signal in signals.forever() {
            println!("Received {:?}", signal);
            match sender.send(signal) {
                Ok(_) => (),
                Err(_) => {
                    println!("Receiver closed");
                    break;
                }
            };
        }
    });

    let my_led = Pin::new(18);
    my_led
        .with_exported(|| {
            my_led.set_direction(Direction::Out).unwrap();
            loop {
                my_led.set_value(0).unwrap();
                thread::sleep(Duration::from_millis(500));
                my_led.set_value(1).unwrap();
                thread::sleep(Duration::from_millis(500));
            }
        })
        .unwrap();
}
