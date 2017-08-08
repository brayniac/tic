// `Allans` is a map for calculating ADEV and AVAR, keyed by metric

use allan::{Allan, Config, Style};
use fnv::FnvHashMap;
use std::hash::Hash;

pub struct Allans<T> {
    config: Config,
    data: FnvHashMap<T, Allan>,
}

impl<T: Hash + Eq> Allans<T> {
    pub fn new() -> Allans<T> {
        let config = Allan::configure().style(Style::AllTau).max_tau(3600);
        Allans {
            config: config,
            data: FnvHashMap::default(),
        }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, self.config.build().unwrap());
    }

    pub fn record(&mut self, key: T, value: f64) {
        if let Some(a) = self.data.get_mut(&key) {
            a.record(value / 1_000_000_000.0); // convert to seconds
            return;
        }
    }

    pub fn adev(&mut self, key: T, tau: usize) -> Option<f64> {
        if let Some(a) = self.data.get(&key) {
            if let Some(t) = a.get(tau) {
                if let Some(adev) = t.deviation() {
                    return Some(adev);
                }
            }
        }
        None
    }
}

#[cfg(feature = "benchmark")]
#[cfg(test)]
mod benchmark {
    extern crate test;
    use super::*;

    #[bench]
    fn init(b: &mut test::Bencher) {
        b.iter(|| {
            let mut allans = Allans::<String>::new();
            allans.init("test".to_owned());
        });
    }

    #[bench]
    fn record(b: &mut test::Bencher) {
        let mut allans = Allans::<String>::new();
        allans.init("test".to_owned());
        b.iter(|| { allans.record("test".to_owned(), 1.0); });
    }
}
