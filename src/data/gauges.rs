// `Gauges` is a map of u64 gauges, keyed by metric

use fnv::FnvHashMap;
use std::hash::Hash;

pub struct Gauges<T> {
    data: FnvHashMap<T, u64>,
}

impl<T: Hash + Eq> Gauges<T> {
    pub fn new() -> Gauges<T> {
        Gauges { data: FnvHashMap::default() }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, 0);
    }

    pub fn remove(&mut self, key: T) {
        self.data.remove(&key);
    }

    #[allow(dead_code)]
    pub fn set(&mut self, key: T, value: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            *h = value;
            return;
        }
    }

    pub fn value(&mut self, key: T) -> u64 {
        if let Some(h) = self.data.get(&key) {
            *h
        } else {
            0
        }
    }
}

#[cfg(feature = "benchmark")]
#[cfg(test)]
mod benchmark {
    extern crate test;
    use super::*;

    #[bench]
    fn new(b: &mut test::Bencher) {
        b.iter(|| Gauges::<String>::new());
    }

    #[bench]
    fn new_init(b: &mut test::Bencher) {
        b.iter(|| {
            let mut counters = Gauges::<String>::new();
            counters.init("test".to_owned());
        });
    }

    #[bench]
    fn set(b: &mut test::Bencher) {
        let mut counters = Gauges::<String>::new();
        counters.init("test".to_owned());
        b.iter(|| { counters.set("test".to_owned(), 42); });
    }
}
