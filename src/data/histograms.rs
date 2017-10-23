// `Histograms` is a map of `Histogram`, keyed by metric

extern crate histogram;

use fnv::FnvHashMap;
use histogram::Histogram;
use std::hash::Hash;

const ONE_SECOND: u64 = 1_000_000_000;
const ONE_MINUTE: u64 = 60 * ONE_SECOND;

pub struct Histograms<T> {
    config: histogram::Config,
    pub data: FnvHashMap<T, Histogram>,
}

impl<T: Hash + Eq> Default for Histograms<T> {
    fn default() -> Histograms<T> {
        Histograms {
            config: Histogram::configure().max_value(ONE_MINUTE),
            data: FnvHashMap::default(),
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

    pub fn remove(&mut self, key: T) {
        self.data.remove(&key);
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

#[cfg(feature = "benchmark")]
#[cfg(test)]
mod benchmark {
    extern crate test;
    use super::*;

    #[bench]
    fn init(b: &mut test::Bencher) {
        b.iter(|| {
            let mut histograms = Histograms::<String>::new();
            histograms.init("test".to_owned());
        });
    }

    #[bench]
    fn increment(b: &mut test::Bencher) {
        let mut histograms = Histograms::<String>::new();
        histograms.init("test".to_owned());
        b.iter(|| { histograms.increment("test".to_owned(), 1); });
    }

    #[bench]
    fn increment_large(b: &mut test::Bencher) {
        let mut histograms = Histograms::<String>::new();
        histograms.init("test".to_owned());
        b.iter(|| { histograms.increment("test".to_owned(), 8_675_309); });
    }
}


#[cfg(test)]
mod test {
    extern crate rand;

    use self::rand::distributions::{IndependentSample, Range};
    use super::*;
    use common::is_between;

    #[test]
    fn white_noise() {
        let mut h = Histograms::<String>::new();
        let key = "test".to_owned();
        h.init(key.clone());

        let mut rng = rand::thread_rng();
        let between = Range::new(1, 100);
        for _ in 0..1_000_000 {
            let v = between.ind_sample(&mut rng);
            h.increment(key.clone(), v);
        }
        for t in vec![25.0, 50.0, 75.0, 90.0, 99.0, 99.9, 99.99] {
            let v = h.percentile(key.clone(), t).unwrap_or_else(|_| {
                println!("error percentile: {}", t);
                panic!("error")
            }) as f64;
            if !is_between(v, t * 0.9, t * 1.1) {
                panic!("percentile: {} value: {} outside of range", t, v);
            }
        }
    }
}
