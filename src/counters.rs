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


#[cfg(test)]
mod tests {
    use super::Counters;

    #[test]
    fn test_new_0() {
        let c = Counters::<usize>::new();
        assert_eq!(c.t0.elapsed().as_secs(), 0);
        assert_eq!(c.get(0), 0);
        assert_eq!(c.get(1), 0);
    }

    #[test]
    fn test_increment_0() {
        let mut c = Counters::<usize>::new();
        assert_eq!(c.get(1), 0);
        c.increment(1);
        assert_eq!(c.get(1), 1);
        assert_eq!(c.get(0), 0);
    }

    #[test]
    fn test_add_0() {
        let mut c = Counters::<usize>::new();
        assert_eq!(c.get(1), 0);
        c.add(1, 100);
        assert_eq!(c.get(1), 100);
        assert_eq!(c.get(0), 0);
    }

    #[test]
    fn test_clear_0() {
        let mut c = Counters::<usize>::new();
        let t0 = c.t0;
        assert_eq!(c.get(1), 0);
        c.add(1, 100);
        assert_eq!(c.get(1), 100);
        c.clear();
        let t1 = c.t0;
        assert!(t0 != t1);
        assert_eq!(c.get(1), 0);
    }

    #[test]
    fn test_rate_0() {
        let mut c = Counters::<usize>::new();
        let t0 = c.t0;
        c.add(1, 1000);
        c.add(2, 2000);
        loop {
            if t0.elapsed().as_secs() >= 1 {
                break;
            }
        }
        let r0 = c.rate(0);
        let r1 = c.rate(1);
        let r2 = c.rate(2);

        assert_eq!(r0, 0.0);
        assert!(r1 > 995.0);
        assert!(r1 < 1005.0);
        assert!(r2 > 1990.0);
        assert!(r2 < 2010.0);
    }
}