use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigPersist {
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
pub struct ValvePersist {
    pub name: String,
    pub pin: u64,
}

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct SchedulePersist {
    pub name: String,
    pub start_offset_min: u64,
    pub duration_min: u64,
    pub repeat_period_days: u64,
    pub valves: Vec<String>,
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
