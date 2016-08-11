use std::fmt::Display;
use std::hash::Hash;
use std::time::Instant;


/// a start and stop time for an event
#[derive(Clone)]
pub struct Sample<T> {
    start: Instant,
    stop: Instant,
    channel: T,
}

impl<T: Hash + Eq + Send + Display + Clone> Sample<T> {
    /// create a new Sample from given start, stop, and channel
    pub fn new(start: Instant, stop: Instant, channel: T) -> Sample<T> {
        Sample {
            start: start,
            stop: stop,
            channel: channel,
        }
    }

    /// return the metric /// deprecated
    pub fn metric(&self) -> T {
        self.channel.clone()
    }

    /// return the metric
    pub fn channel(&self) -> T {
        self.channel.clone()
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
