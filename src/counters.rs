// `Counters` is a map of u64 counters, keyed by metric

use fnv::FnvHashMap;
use std::hash::Hash;

pub struct Counters<T> {
    data: FnvHashMap<T, u64>,
}

impl<T: Hash + Eq> Counters<T> {
    pub fn new() -> Counters<T> {
        Counters { data: FnvHashMap::default() }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, 0);
    }

    #[allow(dead_code)]
    pub fn increment(&mut self, key: T) {
        self.increment_by(key, 1);
    }

    pub fn increment_by(&mut self, key: T, count: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            *h += count;
            return;
        }
    }

    pub fn count(&mut self, key: T) -> u64 {
        if let Some(h) = self.data.get(&key) {
            *h
        } else {
            0
        }
    }
}
