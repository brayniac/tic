// `Meters` hold calculated values

use common::Percentile;
use fnv::FnvHashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;

/// `Meters` are the aggregated result of stats which
/// have been processed by the `Receiver`.
#[derive(Clone)]
pub struct Meters<T> {
    resource_type: PhantomData<T>,
    /// a map of labels to their u64 values
    pub data: FnvHashMap<String, u64>,
    /// a map of labels to their f64 values
    pub data_float: FnvHashMap<String, f64>,
}

impl<T: Hash + Eq> Default for Meters<T> {
    fn default() -> Meters<T> {
        Meters {
            data: FnvHashMap::default(),
            data_float: FnvHashMap::default(),
            resource_type: PhantomData::<T>,
        }
    }
}

impl<T: Hash + Eq + Send + Display + Clone> Meters<T> {
    /// create a new empty set of `Meters`
    pub fn new() -> Meters<T> {
        Default::default()
    }

    /// update the count of events for a given metric
    pub fn set_count(&mut self, channel: T, value: u64) {
        let key = format!("{}_count", channel);
        self.data.insert(key, value);
    }

    /// update the `Percentile` for a given metric
    pub fn set_latency_percentile(&mut self, channel: T, percentile: Percentile, value: u64) {
        let key = format!("{}_{}_nanoseconds", channel, percentile.0);
        self.data.insert(key, value);
    }

    /// update the `Percentile` for a given metric
    pub fn set_value_percentile(&mut self, channel: T, percentile: Percentile, value: u64) {
        let key = format!("{}_{}_units", channel, percentile.0);
        self.data.insert(key, value);
    }

    /// update the Allan Deviation for a given metric at a specific Tau
    pub fn set_adev(&mut self, channel: T, tau: usize, value: f64) {
        let key = format!("{}_tau_{}_adev", channel, tau);
        self.data_float.insert(key, value);
    }

    /// gets the count for a given metric
    pub fn count(&self, channel: &T) -> Option<&u64> {
        let key = format!("{}_count", channel);
        self.data.get(&key)
    }

    /// get a `Percentile` of sample latencies for a given metric
    pub fn latency_percentile(&self, channel: &T, percentile: Percentile) -> Option<&u64> {
        let key = format!("{}_{}_nanoseconds", channel, percentile.0);
        self.data.get(&key)
    }

    /// get the `Percentile` of sample counts for a given metric
    pub fn value_percentile(&self, channel: &T, percentile: Percentile) -> Option<&u64> {
        let key = format!("{}_{}_units", channel, percentile.0);
        self.data.get(&key)
    }

    /// get the Allan Deviation for the channel for a given Tau
    pub fn adev(&self, channel: T, tau: usize) -> Option<&f64> {
        let key = format!("{}_tau_{}_adev", channel, tau);
        self.data_float.get(&key)
    }

    /// clear the Meters
    pub fn clear(&mut self) {
        self.data.clear()
    }
}
