use data::Meters;
use std::sync::mpsc::SyncSender;

/// Duration of 1 second in nanoseconds
pub const SECOND: u64 = 1_000_000_000;
/// Duration of 1 minute in nanoseconds
pub const MINUTE: u64 = 60 * SECOND;

#[derive(Clone, Eq, Hash, PartialEq)]
/// an Interest registers a metric for reporting
pub enum Interest<T> {
    /// Calculate ADEV for the given metric based on the phase difference
    /// between start and stop of each `Sample`. NOTE: It is expected that there
    /// is 1 sample per second per metric
    AllanDeviation(T),
    /// Keep a counter for the given metric, this is incremented by each count
    /// associated with a `Sample`
    Count(T),
    /// Keep a point-in-time value for the given metric, this is last-write-wins
    Gauge(T),
    /// Calculate latency percentiles for metric based on the delta between
    /// start and stop time for each `Sample`
    LatencyPercentile(T),
    /// Calculate value percentiles for metric based on the counts associated
    /// with each `Sample`
    ValuePercentile(T),
    /// Creates a trace file of the latency heatmaps which store the delta
    /// between start and stop time for each `Sample`
    LatencyTrace(T, String),
    /// Generate a PNG plot of the latency heatmaps which store the delta
    /// between start and stop time for each `Sample`
    LatencyWaterfall(T, String),
    /// Creates a trace file of the value heatmaps which store counts
    /// asccociated with each `Sample`
    ValueTrace(T, String),
    /// Generate a PNG plot of the value heatmaps which store counts asccociated
    /// with each `Sample`
    ValueWaterfall(T, String),
}

#[derive(Clone)]
pub enum ControlMessage<T> {
    AddInterest(Interest<T>),
    RemoveInterest(Interest<T>),
    SnapshotMeters(SyncSender<Meters<T>>),
}

#[derive(Clone)]
/// a Percentile is the label plus floating point percentile representation
pub struct Percentile(pub String, pub f64);

// helper function to populate the default `Percentile`s to report
pub fn default_percentiles() -> Vec<Percentile> {
    let mut p = Vec::new();
    p.push(Percentile("min".to_owned(), 0.0));
    p.push(Percentile("p50".to_owned(), 50.0));
    p.push(Percentile("p75".to_owned(), 75.0));
    p.push(Percentile("p90".to_owned(), 90.0));
    p.push(Percentile("p95".to_owned(), 95.0));
    p.push(Percentile("p99".to_owned(), 99.0));
    p.push(Percentile("p999".to_owned(), 99.9));
    p.push(Percentile("p9999".to_owned(), 99.99));
    p.push(Percentile("max".to_owned(), 100.0));
    p
}

// helper function to populate the default `Taus`s to report
// - All Taus from 1 to 300s inclusive
pub fn default_taus() -> Vec<usize> {
    let mut t = Vec::new();
    for i in 1..301 {
        t.push(i);
    }
    t
}

// helper function for tests, ignore dead_code warnings
#[allow(dead_code)]
pub fn is_between(value: f64, min: f64, max: f64) -> bool {
    value >= min && value <= max
}
