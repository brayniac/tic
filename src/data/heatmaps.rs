// `Heatmaps` is a map of `Heatmap`s, keyed by metric

extern crate heatmap;

use fnv::FnvHashMap;
use heatmap::Heatmap;
use std::hash::Hash;
use waterfall::Waterfall;

pub struct Heatmaps<T> {
    config: heatmap::Config,
    pub data: FnvHashMap<T, Heatmap>,
}

impl<T: Hash + Eq> Heatmaps<T> {
    pub fn new(slices: usize, t0: u64) -> Heatmaps<T> {
        let config = Heatmap::configure()
            .slice_duration(1_000_000_000)
            .num_slices(slices)
            .precision(2)
            .start(t0);
        Heatmaps {
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

    pub fn increment(&mut self, key: T, start: u64, value: u64) {
        self.increment_by(key, start, value, 1);
    }

    pub fn increment_by(&mut self, key: T, start: u64, value: u64, count: u64) {
        if let Some(h) = self.data.get_mut(&key) {
            let _ = h.increment_by(start, value, count);
            return;
        }
    }

    pub fn trace(&mut self, key: T, file: String) {
        if let Some(h) = self.data.get_mut(&key) {
            h.save(file);
        }
    }

    pub fn waterfall(&self, key: T, file: String) {
        if let Some(h) = self.data.get(&key) {
            trace!("trace for heatmap with: {} slices", h.num_slices());
            let mut waterfall = Waterfall::new();
            waterfall.render_png(h, file);
        }
    }

    pub fn clear(&mut self) {
        for heatmap in self.data.values_mut() {
            heatmap.clear();
        }
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
            let mut heatmaps = Heatmaps::<String>::new(3600, 0);
            heatmaps.init("test".to_owned());
        });
    }

    #[bench]
    fn increment(b: &mut test::Bencher) {
        let mut heatmaps = Heatmaps::<String>::new(3600, 0);
        heatmaps.init("test".to_owned());
        b.iter(|| { heatmaps.increment("test".to_owned(), 1, 1); });
    }

    #[bench]
    fn increment_large(b: &mut test::Bencher) {
        let mut heatmaps = Heatmaps::<String>::new(3600, 0);
        heatmaps.init("test".to_owned());
        b.iter(|| { heatmaps.increment("test".to_owned(), 1, 8_675_309); });
    }
}
