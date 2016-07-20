// a counter holds counts of events

use std::collections::HashMap;

use std::hash::Hash;

pub struct Gauges<T> {
    pub data: HashMap<T, u64>,
}

impl<T: Hash + Eq> Default for Gauges<T> {
    fn default() -> Gauges<T> {
        Gauges { data: HashMap::new() }
    }
}

impl<T: Hash + Eq> Gauges<T> {
    pub fn new() -> Gauges<T> {
        Default::default()
    }

    pub fn set(&mut self, key: T, value: u64) {
        self.data.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.data = HashMap::new();
    }

    pub fn get(&self, key: T) -> u64 {
        if let Some(c) = self.data.get(&key) {
            return *c;
        }
        0
    }
}
