// a counter holds counts of events

use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;

pub struct Counters<T> {
    pub counts: HashMap<T, u64>,
    pub t0: Instant,
}

impl<T: Hash + Eq> Default for Counters<T> {
    fn default() -> Counters<T> {
        Counters {
            counts: HashMap::new(),
            t0: Instant::now(),
        }
    }
}

impl<T: Hash + Eq> Counters<T> {
    pub fn new() -> Counters<T> {
        Default::default()
    }

    pub fn increment(&mut self, key: T) {
        self.add(key, 1);
    }

    pub fn add(&mut self, key: T, count: u64) {
        if let Some(c) = self.counts.get_mut(&key) {
            *c += count;
            return;
        }
        self.counts.insert(key, count);
    }

    pub fn clear(&mut self) {
        self.counts = HashMap::new();
        self.t0 = Instant::now();
    }

    pub fn get(&self, key: T) -> u64 {
        if let Some(c) = self.counts.get(&key) {
            return *c;
        }
        0
    }

    pub fn percent_of_sum(&self, a: T, b: T) -> f64 {
        let a = self.get(a) as f64;
        let b = self.get(b) as f64;

        let t = a + b;

        if t > 0.0 {
            return 100_f64 * a / t;
        }
        0.0
    }

    pub fn rate(&self, key: T) -> f64 {
        self.get(key) as f64 / (self.t0.elapsed().as_secs() as f64 + self.t0.elapsed().subsec_nanos() as f64 / 1_000_000_000.0)
    }
}
