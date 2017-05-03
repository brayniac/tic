// `Histograms` is a map of `Histogram`, keyed by metric

extern crate histogram;

use histogram::Histogram;
use std::collections::HashMap;
use std::hash::Hash;

const ONE_SECOND: u64 = 1_000_000_000;
const ONE_MINUTE: u64 = 60 * ONE_SECOND;

pub struct Histograms<T> {
    config: histogram::Config,
    pub data: HashMap<T, Histogram>,
}

impl<T: Hash + Eq> Default for Histograms<T> {
    fn default() -> Histograms<T> {
        Histograms {
            config: Histogram::configure().max_value(ONE_MINUTE),
            data: HashMap::new(),
        }
    }
}

impl<T: Hash + Eq> Histograms<T> {
    pub fn new() -> Histograms<T> {
        Default::default()
    }

    pub fn increment(&mut self, key: T, duration: u64) {
        self.increment_by(key, duration, 1);
    }

    pub fn increment_by(&mut self, key: T, duration: u64, count: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            let _ = h.increment_by(duration, count);
            return;
        }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, self.config.build().unwrap());
    }

    pub fn clear(&mut self) {
        for histogram in self.data.values_mut() {
            histogram.clear();
        }
    }

    pub fn percentile(&self, key: T, percentile: f64) -> Result<u64, &'static str> {
        if let Some(h) = self.data.get(&key) {
            return h.percentile(percentile);
        }
        Err("no data")
    }
}


#[cfg(test)]
mod tests {
    use super::Histograms;

    #[test]
    fn test_new_0() {
        let h = Histograms::<usize>::new();
        assert_eq!(h.percentile(0, 50.0), Err("no data"));
        assert_eq!(h.percentile(1, 50.0), Err("no data"));
    }

    #[test]
    fn test_increment_0() {
        let mut h = Histograms::<usize>::new();

        h.init(1);
        for i in 100..200 {
            h.increment(1, i);
        }

        assert_eq!(h.percentile(1, 0.0).unwrap(), 100);
        assert_eq!(h.percentile(1, 10.0).unwrap(), 109);
        assert_eq!(h.percentile(1, 25.0).unwrap(), 124);
        assert_eq!(h.percentile(1, 50.0).unwrap(), 150);
        assert_eq!(h.percentile(1, 75.0).unwrap(), 175);
        assert_eq!(h.percentile(1, 90.0).unwrap(), 190);
        assert_eq!(h.percentile(1, 95.0).unwrap(), 195);
        assert_eq!(h.percentile(1, 100.0).unwrap(), 199);

        assert_eq!(h.percentile(0, 50.0), Err("no data"));
    }
}
