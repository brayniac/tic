use std::fmt::Display;
use std::hash::Hash;
use std::time::Instant;


/// a start and stop time for a metric
#[derive(Clone)]
pub struct Sample<T> {
    start: Instant,
    stop: Instant,
    metric: T,
}

impl<T: Hash + Eq + Send + Display + Clone> Sample<T> {
    /// create a new Sample from given start, stop, and metric
    pub fn new(start: Instant, stop: Instant, metric: T) -> Sample<T> {
        Sample {
            start: start,
            stop: stop,
            metric: metric,
        }
    }

    /// return the metric
    pub fn metric(&self) -> T {
        self.metric.clone()
    }

    /// return the duration
    pub fn duration(&self) -> u64 {
        let d = self.stop.duration_since(self.start);
        d.as_secs() as u64 * 1_000_000_000 + d.subsec_nanos() as u64
    }

    /// return the start time
    pub fn start(&self) -> Instant {
        self.start
    }
}
