// a counter holds counts of events

use std::fmt::Display;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use receiver::Percentile;

#[derive(Clone)]
pub struct Meters<T> {
    pub data: HashMap<String, u64>,
    pub combined: HashMap<String, u64>,
    resource_type: PhantomData<T>,
}

impl<T: Hash + Eq> Default for Meters<T> {
    fn default() -> Meters<T> {
        Meters {
            data: HashMap::new(),
            combined: HashMap::new(),
            resource_type: PhantomData::<T>,
        }
    }
}

impl<T: Hash + Eq + Send + Display + Clone> Meters<T> {
    pub fn new() -> Meters<T> {
        Default::default()
    }

    pub fn set_count(&mut self, channel: T, value: u64) {
        let key = format!("{}_count", channel);
        self.data.insert(key, value);
    }

    pub fn set_combined_count(&mut self, value: u64) {
        let key = "count".to_owned();
        self.combined.insert(key, value);
    }

    pub fn set_percentile(&mut self, channel: T, percentile: Percentile, value: u64) {
        let key = format!("{}_{}_nanoseconds", channel, percentile.0);
        self.data.insert(key, value);
    }

    pub fn set_combined_percentile(&mut self, percentile: Percentile, value: u64) {
        let key = format!("{}_nanoseconds", percentile.0);
        self.combined.insert(key, value);
    }

    pub fn get_count(&self, channel: &T) -> Option<&u64> {
        let key = format!("{}_count", channel);
        self.data.get(&key)
    }

    pub fn get_combined_count(&self) -> Option<&u64> {
        let key = "count".to_owned();
        self.combined.get(&key)
    }

    pub fn get_percentile(&self, channel: &T, percentile: Percentile) -> Option<&u64> {
        let key = format!("{}_{}_nanoseconds", channel, percentile.0);
        self.data.get(&key)
    }

    pub fn get_combined_percentile(&self, percentile: Percentile) -> Option<&u64> {
        let key = format!("{}_nanoseconds", percentile.0);
        self.combined.get(&key)
    }
}
