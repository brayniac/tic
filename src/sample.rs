use std::fmt::Display;
use std::hash::Hash;

/// a start and stop time for an event
#[derive(Clone)]
pub struct Sample<T> {
    start: u64,
    stop: u64,
    channel: T,
}

impl<T: Hash + Eq + Send + Display + Clone> Sample<T> {
    /// create a new Sample from given start, stop, and channel
    pub fn new(start: u64, stop: u64, channel: T) -> Sample<T> {
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
        self.stop - self.start
    }

    /// return the start time
    pub fn start(&self) -> u64 {
        self.start
    }

    /// return the stop time
    pub fn stop(&self) -> u64 {
        self.stop
    }
}
