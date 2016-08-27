// `Heatmaps` is a map of `Heatmap`s, keyed by metric

extern crate heatmap;
extern crate waterfall;

use heatmap::Heatmap;
use waterfall::Waterfall;

use std::collections::HashMap;
use std::hash::Hash;

pub struct Heatmaps<T> {
    config: heatmap::Config,
    pub metric: HashMap<T, Heatmap>,
    pub total: Heatmap,
    pub t0: u64,
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
            metric: HashMap::new(),
            total: config.build().unwrap(),
            t0: t0,
        }
    }

    pub fn increment(&mut self, key: T, start: u64, value: u64) {
        self.increment_by(key, start, value, 1);
    }

    pub fn increment_by(&mut self, key: T, start: u64, value: u64, count: u64) {
        let _ = self.total.increment_by(start, value, count);
        if let Some(h) = self.metric.get_mut(&key) {
            let _ = h.increment_by(start, value, count);
            return;
        }
        let mut h = self.config.build().unwrap();
        let _ = h.increment_by(start, value, count);
        self.metric.insert(key, h);
    }

    pub fn metric_trace(&mut self, key: T, file: String) {
        if let Some(h) = self.metric.get_mut(&key) {
            h.save(file);
        }
    }

    pub fn metric_waterfall(&self, key: T, file: String) {
        if let Some(h) = self.metric.get(&key) {
            info!("trace for heatmap with: {} slices", h.num_slices());
            let mut waterfall = Waterfall::new();
            waterfall.render_png(h, file);
        }
    }

    pub fn total_trace(&self, file: String) {
        self.total.save(file);
    }

    pub fn total_waterfall(&self, file: String) {
        let mut waterfall = Waterfall::new();
        waterfall.render_png(&self.total, file);
    }

    pub fn metric_count(&mut self, key: T) -> u64 {
        if let Some(h) = self.metric.get(&key) {
            return h.clone().entries();
        }
        0
    }

    pub fn total_count(&mut self) -> u64 {
        self.total.entries()
    }
}


// #[cfg(test)]
// mod tests {
//     use super::Histograms;

//     #[test]
//     fn test_new_0() {
//         let h = Histograms::<usize>::new();
//         assert_eq!(h.t0.elapsed().as_secs(), 0);
//         assert_eq!(h.metric_percentile(0, 50.0), Err("no data"));
//         assert_eq!(h.metric_percentile(1, 50.0), Err("no data"));
//     }

//     #[test]
//     fn test_increment_0() {
//         let mut h = Histograms::<usize>::new();

//         for i in 100..200 {
//             h.increment(1, i);
//         }

//         assert_eq!(h.metric_percentile(1, 0.0).unwrap(), 100);
//         assert_eq!(h.metric_percentile(1, 10.0).unwrap(), 109);
//         assert_eq!(h.metric_percentile(1, 25.0).unwrap(), 124);
//         assert_eq!(h.metric_percentile(1, 50.0).unwrap(), 150);
//         assert_eq!(h.metric_percentile(1, 75.0).unwrap(), 175);
//         assert_eq!(h.metric_percentile(1, 90.0).unwrap(), 190);
//         assert_eq!(h.metric_percentile(1, 95.0).unwrap(), 195);
//         assert_eq!(h.metric_percentile(1, 100.0).unwrap(), 199);

//         assert_eq!(h.metric_percentile(0, 50.0), Err("no data"));
//     }
// }
