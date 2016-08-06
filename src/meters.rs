// a counter holds counts of events

use std::collections::HashMap;

use std::hash::Hash;

pub struct Meters<T> {
    pub data: HashMap<T, u64>,
}

impl<T: Hash + Eq> Default for Meters<T> {
    fn default() -> Meters<T> {
        Meters { data: HashMap::new() }
    }
}

impl<T: Hash + Eq> Meters<T> {
    pub fn new() -> Meters<T> {
        Default::default()
    }

    pub fn set(&mut self, key: T, value: u64) {
        self.data.insert(key, value);
    }
}
