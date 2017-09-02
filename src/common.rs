#[derive(Clone, Eq, Hash, PartialEq)]
/// an Interest registers a metric for reporting
pub enum Interest<T> {
    AllanDeviation(T),
    Count(T),
    Percentile(T),
    Trace(T, String),
    Waterfall(T, String),
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum ControlMessage<T> {
    AddInterest(Interest<T>),
    RemoveInterest(Interest<T>),
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
