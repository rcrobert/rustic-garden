use std::thread;
use std::time::Duration;
use sysfs_gpio::{Direction, Pin};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Valves {
    valves: HashMap<String, Valve>,
}

impl Valves {

    /// Gets a valve by name.
    pub fn get(&self, name: &str) -> Option<&Valve> {
        self.valves.get(name)
    }

    /// Gets a valve by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Valve> {
        self.valves.get_mut(name)
    }

    /// Registers a new valve connected to the given GPIO pin.
    pub fn register_new_valve(&mut self, name: String, pin: u64) {
        self.valves.insert(name.clone(), Valve::new(name, pin));
    }
}

#[derive(Debug)]
pub struct Valve {
    name: String,
    id: u64,
    pin: OutputPin,
}

/// The possible states of a controlled valve.
pub enum ValveState {
    /// The valve is open.
    Open,

    /// The valve is closed.
    Closed,
}

impl Valve {
    /// Creates a new valve connected to the given GPIO pin.
    pub fn new(name: String, pin: u64) -> Valve {
        let output_pin = OutputPin::export(pin).expect("exporting pin will not fail");
        Valve {
            name,
            id: pin,
            pin: output_pin,
        }
    }

    /// Opens the valve.
    pub fn open(&mut self) -> Result<()> {
        self.pin.set_value(1)
    }

    /// Closes the valve.
    pub fn close(&mut self) -> Result<()> {
        self.pin.set_value(0)
    }

    /// Retrieves the current valve state.
    pub fn get_state(&self) -> Result<ValveState> {
        let value = self.pin.get_value()?;
        match value {
            0 => Ok(ValveState::Closed),
            _ => Ok(ValveState::Open),
        }
    }
}

pub type Error = sysfs_gpio::Error;
pub type Result<T> = sysfs_gpio::Result<T>;

/// RAII guard for a `sysfs_gpio::Pin`.
///
/// This ensures the underlying `Pin` is cleaned up and set back to input automatically. It also
/// encapsulates all of the sysfs_gpio behavior, the public layer above handles error translation.
#[derive(Debug)]
struct OutputPin {
    pin_number: u64,
    pin: Pin,
}

impl OutputPin {
    fn export(pin_number: u64) -> sysfs_gpio::Result<OutputPin> {
        let pin = Pin::new(pin_number);
        pin.export()?;
        OutputPin::try_set_direction(&pin)?;
        Ok(OutputPin { pin_number, pin })
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
        self.pin.set_value(value)
    }

    fn get_value(&self) -> sysfs_gpio::Result<u8> {
        self.pin.get_value()
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
