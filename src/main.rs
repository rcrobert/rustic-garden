extern crate sysfs_gpio;

use signal_hook;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;
use std::time::Duration;
use sysfs_gpio::{Direction, Pin};

fn main() {
    let (sender, receiver) = channel::<i32>();

    // Listen for SIGINT for graceful shutdown
    thread::spawn(move || {
        let signals = signal_hook::iterator::Signals::new(&[signal_hook::SIGINT])
            .expect("Could not subscribe to signals");

        // Only interested in the first SIGINT anyways
        for signal in signals.wait() {
            println!("Sent {:?}", signal);
            if let Err(_) = sender.send(signal) {
                println!("Receiver closed");
            }
        }
        signals.close();
    });

    let my_led = OutputPin::new(18).unwrap();
    loop {
        if let Err(e) = my_led.set_value(0) {
            println!("{:?}", e);
        }
        thread::sleep(Duration::from_millis(500));

        if let Err(e) = my_led.set_value(1) {
            println!("{:?}", e);
        }
        thread::sleep(Duration::from_millis(500));

        match receiver.try_recv() {
            Ok(sig) => {
                // We were interrupted
                println!("Received {:?}", sig);
                break;
            }
            Err(err) => {
                if let TryRecvError::Disconnected = err {
                    // Receiver is gone
                    break;
                }
            }
        }
    }
}

#[derive(Debug)]
struct OutputPin {
    id: u64,
    pin: Pin,
}

impl OutputPin {
    fn new(id: u64) -> sysfs_gpio::Result<Self> {
        let pin = Pin::new(id);
        pin.export()?;
        OutputPin::try_set_direction(&pin)?;
        Ok(Self { id, pin })
    }

    fn try_set_direction(pin: &Pin) -> sysfs_gpio::Result<()> {
        // Retry first access to newly exported GPIO
        // GPIO permissions are configured asynchronously by udev
        // See: https://github.com/rust-embedded/rust-sysfs-gpio/issues/5
        let mut failures = 0;
        while let Err(e) = pin.set_direction(Direction::Out) {
            thread::sleep(Duration::from_millis(10));
            failures += 1;
            if failures > 10 {
                return Err(e);
            }
        }

        Ok(())
    }

    fn set_value(&self, value: u8) -> sysfs_gpio::Result<()> {
        self.pin.set_value(value)?;
        Ok(())
    }
}

// RAII for OutputPin shutdown
impl Drop for OutputPin {
    fn drop(&mut self) {
        if let Err(err) = self.pin.set_direction(Direction::In) {
            println!("Failed to set input {:?}: {:?}", self, err);
        }
        if let Err(err) = self.pin.unexport() {
            println!("Failed to unexport {:?}: {:?}", self, err);
        }
    }
}
