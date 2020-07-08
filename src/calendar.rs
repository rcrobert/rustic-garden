extern crate log;

use log::{error, info};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io;
use std::io::{BufReader, Read, Write};

#[derive(Clone, Debug)]
pub struct Schedule {
    name: String,
    start_offset_min: u64,
    duration_min: u64,
    repeat_period_days: u64,
    valves: Vec<String>,
}

pub struct Calendar {
    cache: ConfigPersist,
    persistent_store: Box<dyn Write>,
}

impl Calendar {
    /// Creates a new, empty Calendar.
    pub fn new(persistent_store: Box<dyn Write>) -> Calendar {
        Calendar {
            cache: ConfigPersist::new(String::from("0.1")),
            persistent_store,
        }
    }

    /// Add a new schedule or overwrite an existing one with the same name
    pub fn create_or_replace_schedule(&mut self, schedule: Schedule) -> io::Result<()> {
        info!(
            "create or replace schedule {}: {:?}",
            schedule.name, schedule
        );
        self.cache.create_or_replace_schedule(schedule.into());
        self.sync()
    }

    pub fn delete_schedule(&mut self, name: &str) -> io::Result<()> {
        info!("delete schedule {}", name);
        self.cache.delete_schedule(name);
        self.sync()
    }

    pub fn list(&self) -> impl Iterator<Item = Schedule> + '_ {
        return self
            .cache
            .iter_schedules()
            .map(|schedule_persist| Schedule::from(schedule_persist.clone()));
    }

    pub fn initialize(&mut self, source: &mut dyn Read) -> io::Result<()> {
        let reader = BufReader::new(source);

        // Deserialize
        let r = serde_yaml::from_reader(reader);
        if let Err(e) = r {
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
        let value = r.unwrap();

        return match serde_yaml::from_value(value) {
            Ok(data) => {
                self.cache = data;
                Ok(())
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        };
    }
}

impl Calendar {
    /// Syncs the in-memory schedules cache to persistent storage.
    fn sync(&mut self) -> io::Result<()> {
        // Convert to serde_yaml
        let r = serde_yaml::to_value(&self.cache);
        if let Err(e) = r {
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
        let value = r.unwrap();

        // Serialize
        let data = serde_yaml::to_string(&value);
        if let Err(e) = data {
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
        let data: String = data.unwrap();

        // Return the result of writing to storage
        return self.persistent_store.write_all(data.as_bytes());
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ConfigPersist {
    version: String,
    valves: Vec<ValvePersist>,

    /// All configured schedules, sorted by name
    schedules: Vec<SchedulePersist>,
}

impl ConfigPersist {
    /// Creates a new, empty persist cache.
    pub fn new(version: String) -> ConfigPersist {
        ConfigPersist {
            version,
            valves: Vec::new(),
            schedules: Vec::new(),
        }
    }

    pub fn create_or_replace_schedule(&mut self, schedule: SchedulePersist) {
        let r = self
            .schedules
            .binary_search_by(|s| schedule.name.cmp(&s.name));
        match r {
            // Exists, replace the entry
            Ok(idx) => {
                let existing_schedule = self.schedules.get_mut(idx).expect("search in bounds");
                *existing_schedule = schedule.into();
            }

            // New entry, the index is where it can be inserted to maintain sorted
            Err(idx) => {
                self.schedules.insert(idx, schedule.into());
                // TODO add is_sorted feature for
                // assert!(self.schedules.is_sorted());
            }
        }
    }

    /// Remove the schedule by name if it exists.
    pub fn delete_schedule(&mut self, name: &str) {
        let r = self.schedules.binary_search_by(|s| name.cmp(&s.name));
        match r {
            Ok(idx) => {
                self.schedules.remove(idx);
            }
            Err(_) => (),
        }
    }

    pub fn iter_schedules(&self) -> impl Iterator<Item = &SchedulePersist> {
        self.schedules.iter()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ValvePersist {
    name: String,
    pin: u64,
}

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
struct SchedulePersist {
    name: String,
    start_offset_min: u64,
    duration_min: u64,
    repeat_period_days: u64,
    valves: Vec<String>,
}

impl Ord for SchedulePersist {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for SchedulePersist {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SchedulePersist {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl From<SchedulePersist> for Schedule {
    fn from(p: SchedulePersist) -> Schedule {
        Schedule {
            name: p.name,
            start_offset_min: p.start_offset_min,
            duration_min: p.duration_min,
            repeat_period_days: p.repeat_period_days,
            valves: p.valves,
        }
    }
}

impl From<Schedule> for SchedulePersist {
    fn from(p: Schedule) -> SchedulePersist {
        SchedulePersist {
            name: p.name,
            start_offset_min: p.start_offset_min,
            duration_min: p.duration_min,
            repeat_period_days: p.repeat_period_days,
            valves: p.valves,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{mem, ptr};

    #[test]
    fn create_and_list_new_schedule() {
        let mut c = Calendar::new(Box::new(Vec::<u8>::new()));
        let schedule_name = String::from("test schedule");
        let new_schedule = any_schedule(&schedule_name);

        c.create_or_replace_schedule(new_schedule.clone());

        assert!(c.list().find(|s| schedule_name == s.name).is_some());
    }

    #[test]
    fn delete_schedule() {
        let mut c = Calendar::new(Box::new(Vec::<u8>::new()));
        let schedule_name = String::from("test schedule");
        let new_schedule = any_schedule(&schedule_name);

        assert!(c.create_or_replace_schedule(new_schedule.clone()).is_ok());
        assert!(c.list().find(|s| schedule_name == s.name).is_some());

        assert!(c.delete_schedule(&schedule_name).is_ok());
        assert!(c.list().find(|s| schedule_name == s.name).is_none());
    }

    #[test]
    fn create_syncs() {
        let mut c = Calendar::new(Box::new(Vec::<u8>::new()));
        let schedule_name = String::from("test schedule");
        let new_schedule = any_schedule(&schedule_name);

        assert!(c.create_or_replace_schedule(new_schedule.clone()).is_ok());

        let p = peek_config_persist(&mut c);

        assert!(p.iter_schedules().find(|s| s.name == schedule_name).is_some());
    }

    #[test]
    fn delete_syncs() {
        let mut c = Calendar::new(Box::new(Vec::<u8>::new()));
        let schedule_name = String::from("test schedule");
        let new_schedule = any_schedule(&schedule_name);

        assert!(c.create_or_replace_schedule(new_schedule.clone()).is_ok());

        // Writing to the vec isnt perfect, it does not clear the tail if it writes less
        clear_storage(&mut c);

        assert!(c.delete_schedule(&schedule_name).is_ok());

        let p = peek_config_persist(&mut c);
        assert!(p.iter_schedules().find(|s| s.name == schedule_name).is_none());
    }

    fn any_schedule(name: &String) -> Schedule {
        Schedule {
            name: name.clone(),
            start_offset_min: 1440,
            duration_min: 60,
            repeat_period_days: 3,
            valves: Vec::new(),
        }
    }

    fn clear_storage(calendar: &mut Calendar) {
        // Swap in an empty Box, let the original drop here
        let original = mem::replace(&mut calendar.persistent_store, Box::new(Vec::new()));
    }

    fn peek_config_persist(calendar: &mut Calendar) -> ConfigPersist {
        let mock_storage = peek_storage(calendar);
        let value = match serde_yaml::from_slice(mock_storage.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                println!("{:?}", String::from_utf8(mock_storage.clone()));
                panic!("deserialize mock storage succeeds");
            },
        };
        return serde_yaml::from_value(value).expect("decode mock storage succeeds");
    }

    fn peek_storage(calendar: &mut Calendar) -> &Vec<u8> {
        // Swap in a temporary Box
        let original = mem::replace(&mut calendar.persistent_store, Box::new(Vec::new()));

        // Capture the raw ptr to yield
        let storage_ptr = Box::into_raw(original) as *mut Vec<u8>;
        assert_ne!(storage_ptr, ptr::null_mut());

        // Rebox it, restore the Logbook
        let original = unsafe { Box::from_raw(storage_ptr) };
        mem::replace(&mut calendar.persistent_store, original);

        return unsafe { &*storage_ptr };
    }
}
