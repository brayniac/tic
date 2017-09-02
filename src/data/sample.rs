use std::fmt::Display;
use std::hash::Hash;

/// a start and stop time for an event
#[derive(Clone)]
pub struct Sample<T> {
    start: u64,
    stop: u64,
    count: u64,
    channel: T,
}

impl<T: Hash + Eq + Send + Display + Clone> Sample<T> {
    /// Create a new Sample for a single event
    pub fn new(start: u64, stop: u64, channel: T) -> Sample<T> {
        Sample {
            start: start,
            stop: stop,
            count: 1,
            channel: channel,
        }
    }

    /// Create a new Sample when multiple occurances of the event have happened
    pub fn counted(start: u64, stop: u64, count: u64, channel: T) -> Sample<T> {
        Sample {
            start: start,
            stop: stop,
            count: count,
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

    /// return the count of events
    pub fn count(&self) -> u64 {
        self.count
    }
}
