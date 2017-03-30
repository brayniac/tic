// `Counters` is a map of u64 counters, keyed by metric

use std::collections::HashMap;
use std::hash::Hash;

pub struct Counters<T> {
    pub metric: HashMap<T, u64>,
    pub total: u64,
}

impl<T: Hash + Eq> Counters<T> {
    pub fn new() -> Counters<T> {
        Counters {
            metric: HashMap::new(),
            total: 0,
        }
    }

    pub fn increment(&mut self, key: T) {
        self.increment_by(key, 1);
    }

    pub fn increment_by(&mut self, key: T, count: u64) {
        self.total += count;
        if let Some(h) = self.metric.get_mut(&key) {
            *h += count;
            return;
        }
        self.metric.insert(key, count);
    }

    pub fn metric_count(&mut self, key: T) -> u64 {
        if let Some(h) = self.metric.get(&key) {
            return h.clone();
        }
        0
    }

    pub fn total_count(&mut self) -> u64 {
        self.total
    }
}
