// histograms is a map of histogram, keyed by status
extern crate histogram;

use histogram::Histogram;

use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;

pub struct Histograms<T> {
    config: histogram::Config,
    pub histograms: HashMap<T, Histogram>,
    pub t0: Instant,
}

impl<T: Hash + Eq> Default for Histograms<T> {
    fn default() -> Histograms<T> {
        Histograms {
            config: Histogram::configure(),
            histograms: HashMap::new(),
            t0: Instant::now(),
        }
    }
}

impl<T: Hash + Eq> Histograms<T> {
    pub fn new() -> Histograms<T> {
        Default::default()
    }

    pub fn increment(&mut self, key: T, duration: u64) {
        self.add(key, duration, 1);
    }

    pub fn add(&mut self, key: T, duration: u64, count: u64) {
        if let Some(h) = self.histograms.get_mut(&key) {
            let _ = h.increment_by(duration, count);
            return;
        }
        let mut h = self.config.clone().build().unwrap();
        let _ = h.increment_by(duration, count);
        self.histograms.insert(key, h);
    }

    pub fn clear(&mut self) {
        self.histograms = HashMap::new();
        self.t0 = Instant::now();
    }

    pub fn get_percentile(&self, key: T, percentile: f64) -> Result<u64, &'static str> {
        if let Some(h) = self.histograms.get(&key) {
            return h.percentile(percentile)
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
        assert_eq!(h.t0.elapsed().as_secs(), 0);
        assert_eq!(h.get_percentile(0, 50.0), Err("no data"));
        assert_eq!(h.get_percentile(1, 50.0), Err("no data"));
    }

    #[test]
    fn test_increment_0() {
        let mut h = Histograms::<usize>::new();

        for i in 100..200 {
            h.increment(1, i);
        }

        assert_eq!(h.get_percentile(1, 0.0).unwrap(), 100);
        assert_eq!(h.get_percentile(1, 10.0).unwrap(), 109);
        assert_eq!(h.get_percentile(1, 25.0).unwrap(), 124);
        assert_eq!(h.get_percentile(1, 50.0).unwrap(), 150);
        assert_eq!(h.get_percentile(1, 75.0).unwrap(), 175);
        assert_eq!(h.get_percentile(1, 90.0).unwrap(), 190);
        assert_eq!(h.get_percentile(1, 95.0).unwrap(), 195);
        assert_eq!(h.get_percentile(1, 100.0).unwrap(), 199);

        assert_eq!(h.get_percentile(0, 50.0), Err("no data"));
    }
}
