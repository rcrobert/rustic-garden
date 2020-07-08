extern crate log;

use chrono::Local;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::io;
use std::io::{BufReader, Read, Write};

/// Structure for tracking runs and completions of schedules.
pub struct Logbook {
    cache: LogbookData,
    backing: Box<dyn Write>,
}

impl Logbook {
    /// Create a new, empty logbook.
    pub fn new(backing: Box<dyn Write>) -> Self {
        Logbook {
            cache: LogbookData::new(),
            backing,
        }
    }

    /// Marks the schedule as started, recording the current time as the start time.
    pub fn mark_started(&mut self, schedule_name: &String) -> io::Result<()> {
        let now: String = Local::now().to_rfc2822();

        info!("marking {} as started at {}", schedule_name, now);

        let mut new_record = Record::new(schedule_name.clone());
        new_record.started = Some(now.clone());

        // Persist the new
        self.cache.records.push(new_record);

        let result = self.sync();
        info!("{} started at {}", schedule_name, now);
        return result;
    }

    /// Marks the schedule as completed, recording the current time as the finish time.
    pub fn mark_completed(&mut self, schedule_name: &String) -> io::Result<()> {
        let now: String = Local::now().to_rfc2822();

        info!("marking {} as completed at {}", schedule_name, now);

        // Find the record for start time
        if let Some(record) = self.cache.find_most_recent_mut(schedule_name) {
            // Ensure it was not already marked as complete
            if let Some(v) = &record.completed {
                error!(
                    "record for {} was already completed at {}",
                    schedule_name, v
                );
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "already completed",
                ));
            }

            // Persist the completion time
            record.completed = Some(now.clone());

            let result = self.sync();
            info!("{} completed at {}", schedule_name, now);
            return result;
        } else {
            error!("no record for {} found, never started", schedule_name);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "never started"));
        }
    }

    /// Initializes the in-memory records cache, usually on on upstart.
    pub fn initialize(&mut self, source: &mut dyn Read) -> io::Result<()> {
        let reader = BufReader::new(source);

        // Deserialize
        let r = serde_yaml::from_reader(reader);
        if let Err(e) = r {
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
        let value: Value = r.unwrap();

        return match serde_yaml::from_value(value) {
            Ok(data) => {
                self.cache = data;
                Ok(())
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e)),
        };
    }

    /// Returns an iterator over the records.
    pub fn iter(&self) -> Iter {
        return Iter::new(self);
    }

    /// Returns an iterator over the incomplete records.
    pub fn iter_incomplete<'a>(&'a self) -> impl Iterator<Item = &'a Record> {
        return self.iter().filter(|&record| match record.completed {
            None => true,
            _ => false,
        });
    }
}

impl Logbook {
    /// Syncs the in-memory records cache to persistent storage.
    fn sync(&mut self) -> io::Result<()> {
        // Convert to serde_yaml
        let r = serde_yaml::to_value(&self.cache);
        if let Err(e) = r {
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
        let value: Value = r.unwrap();

        // Serialize
        let data = serde_yaml::to_string(&value);
        if let Err(e) = data {
            return Err(io::Error::new(io::ErrorKind::InvalidData, e));
        }
        let data: String = data.unwrap();

        // Return the result of writing to storage
        return self.backing.write_all(data.as_bytes());
    }
}

pub struct Iter<'a> {
    data: &'a Logbook,
    i: usize,
}

impl<'a> Iter<'a> {
    fn new(data: &'a Logbook) -> Self {
        Self { data, i: 0 }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a Record;
    fn next(&mut self) -> Option<Self::Item> {
        let r = self.data.cache.records.get(self.i);
        self.i += 1;
        return r;
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct LogbookData {
    records: Vec<Record>,
}

impl LogbookData {
    fn new() -> Self {
        Self { records: vec![] }
    }

    /// Finds the most recent `Record` for a schedule by name.
    ///
    /// Returns `None` if there are no records for the given schedule.
    fn find_most_recent(&self, name: &String) -> Option<&Record> {
        return self.records.iter().rfind(|record| record.name == *name);
    }

    /// Finds the most recent `Record` for a schedule by name.
    ///
    /// Returns `None` if there are no records for the given schedule.
    fn find_most_recent_mut(&mut self, name: &String) -> Option<&mut Record> {
        return self.records.iter_mut().rfind(|record| record.name == *name);
    }
}

/// A record of when a schedule was started and completed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// The name of the schedule this record tracks.
    pub name: String,

    /// The time when this schedule was started or `None` if it was not started.
    pub started: Option<String>,

    /// The time when this schedule was completed or `None` if it was not completed.
    pub completed: Option<String>,
}

impl Record {
    fn new(name: String) -> Self {
        Self {
            name,
            started: None,
            completed: None,
        }
    }
}

impl From<&str> for Record {
    fn from(s: &str) -> Self {
        Self::new(String::from(s))
    }
}

impl From<String> for Record {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::{mem, ptr};

    #[test]
    fn new_record_started_field_initialized_to_null() {
        let record = Record::from("any name");
        assert_eq!(record.started, None);
    }

    #[test]
    fn new_record_completed_field_initialized_to_null() {
        let record = Record::from("any name");
        assert_eq!(record.started, None);
    }

    /// Tests that the logbook `mark_started` syncs to persistent storage by checking the storage
    /// is not empty afterwards.
    #[test]
    fn logbook_mark_started_syncs() {
        let mut l = Logbook::new(Box::new(Vec::<u8>::new()));

        let schedule = String::from("any schedule");
        l.mark_started(&String::from("any schedule"))
            .expect("mark_started succeeds");

        let d = peek_logbook(&mut l);
        let record = d.find_most_recent(&schedule);
        assert!(record.is_some());

        let record = record.unwrap();
        assert!(record.started.is_some());
    }

    /// Tests that the logbook `mark_completed` syncs to persistent storage by checking the storage
    /// is not empty afterwards.
    #[test]
    fn logbook_mark_completed_syncs() {
        let mut l = Logbook::new(Box::new(Vec::<u8>::new()));

        let schedule = String::from("any schedule");
        l.mark_started(&String::from("any schedule"))
            .expect("mark_started succeeds");
        l.mark_completed(&String::from("any schedule"))
            .expect("mark_completed succeeds");

        let d = peek_logbook(&mut l);
        let record = d.find_most_recent(&schedule);
        assert!(record.is_some());

        let record = record.unwrap();
        assert!(record.completed.is_some());
    }

    #[test]
    fn test_mark_completed_of_unstarted_schedule_fails() {
        let mut l = Logbook::new(Box::new(Vec::<u8>::new()));

        assert!(l.mark_completed(&String::from("any schedule")).is_err());
    }

    #[test]
    fn test_mark_completed_of_unstarted_schedule_does_not_sync() {
        let mut l = Logbook::new(Box::new(Vec::<u8>::new()));

        l.mark_completed(&String::from("any schedule")).expect_err("mark_completed fails");

        assert_eq!(peek_storage(&mut l).len(), 0);
    }

    #[test]
    fn test_mark_completed_of_already_completed_schedule_fails() {
        let schedule = String::from("any schedule");
        let mut l = Logbook::new(Box::new(Vec::<u8>::new()));

        l.mark_started(&schedule).expect("mark_started succeeds");
        l.mark_completed(&schedule)
            .expect("first mark_completed succeeds");

        assert!(l.mark_completed(&schedule).is_err());
    }

    /// Helper to peek at the internal `Logbook` storage
    fn peek_logbook(logbook: &mut Logbook) -> LogbookData {
        let mock_storage = peek_storage(logbook);
        let value = serde_yaml::from_slice(mock_storage.as_slice())
            .expect("deserialize mock storage succeeds");
        return serde_yaml::from_value(value).expect("decode mock storage succeeds");
    }

    /// Helper to peek at the internal `Logbook` raw storage
    fn peek_storage(l: &mut Logbook) -> &Vec<u8> {
        // Swap in a temporary Box
        let original = mem::replace(&mut l.backing, Box::new(Vec::<u8>::new()));

        // Capture the raw ptr to yield
        let storage_ptr = Box::into_raw(original) as *mut Vec<u8>;
        assert_ne!(storage_ptr, ptr::null_mut());

        // Rebox it, restore the Logbook
        let original = unsafe { Box::from_raw(storage_ptr) };
        mem::replace(&mut l.backing, original);

        return unsafe { &*storage_ptr };
    }
}
