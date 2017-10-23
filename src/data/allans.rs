// `Allans` is a map for calculating ADEV and AVAR, keyed by metric

use SECOND;
use allan::{Allan, Config, Style};
use fnv::FnvHashMap;
use std::hash::Hash;

pub struct Allans<T> {
    config: Config,
    data: FnvHashMap<T, Allan>,
}

impl<T: Hash + Eq> Allans<T> {
    pub fn new(max_tau: usize) -> Allans<T> {
        let config = Allan::configure().max_tau(max_tau).style(Style::AllTau);
        Allans {
            config: config,
            data: FnvHashMap::default(),
        }
    }

    pub fn init(&mut self, key: T) {
        self.data.insert(key, self.config.build().unwrap());
    }

    pub fn remove(&mut self, key: T) {
        self.data.remove(&key);
    }

    pub fn record(&mut self, key: T, value: f64) {
        if let Some(a) = self.data.get_mut(&key) {
            a.record(value / SECOND as f64); // convert nanoseconds to seconds
        }
    }

    pub fn adev(&mut self, key: &T, tau: usize) -> Result<f64, &'static str> {
        let allan = self.data.get(key).ok_or("key not found")?;
        let tau = allan.get(tau).ok_or("no tau for allan")?;
        tau.deviation().ok_or("no adev for tau")
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

#[cfg(test)]
mod test {
    extern crate rand;

    use self::rand::distributions::{IndependentSample, Range};
    use super::*;
    use common::is_between;

    #[test]
    fn white_noise() {
        let mut allans = Allans::<String>::new(1000);
        let key = "test".to_owned();
        allans.init(key.clone());

        let mut rng = rand::thread_rng();
        let between = Range::new(0.0, 1.0);
        for _ in 0..10_000 {
            let v = between.ind_sample(&mut rng);
            allans.record(key.clone(), v);
        }
        for t in 1..1000 {
            let v = allans.adev(&key, t).unwrap_or_else(|e| {
                println!("error fetching for tau: {} error: {}", t, e);
                panic!("error")
            }) * t as f64;
            if !is_between(v, 4e-10, 6e-10) {
                panic!("tau: {} value: {} outside of range", t, v);
            }
        }
    }
}
