#![allow(deprecated)]

use clocksource::Clocksource;
use common::{self, ControlMessage, Interest, Percentile};
use config::Config;
use controller::Controller;
use data::{Allans, Counters, Gauges, Heatmaps, Histograms, Meters, Sample};
use mio::{self, Events, Poll, PollOpt, Ready};
use mio_extras::channel;
use mpmc::Queue;
use sender::Sender;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;

// define token numbers for data and control queues
#[derive(Clone, Copy)]
enum Token {
    Control = 0,
    Data = 1,
}

/// a `Receiver` processes incoming `Sample`s and generates stats
pub struct Receiver<T> {
    window_time: u64,
    window_duration: u64,
    end_time: u64,
    run_duration: u64,
    config: Config<T>,
    empty_queue: Arc<Queue<Vec<Sample<T>>>>,
    data_rx: channel::Receiver<Vec<Sample<T>>>,
    data_tx: channel::SyncSender<Vec<Sample<T>>>,
    control_rx: channel::Receiver<ControlMessage<T>>,
    control_tx: channel::SyncSender<ControlMessage<T>>,
    allans: Allans<T>,
    counters: Counters<T>,
    gauges: Gauges<T>,
    latency_histograms: Histograms<T>,
    value_histograms: Histograms<T>,
    meters: Meters<T>,
    interests: HashSet<Interest<T>>,
    taus: Vec<usize>,
    percentiles: Vec<Percentile>,
    latency_heatmaps: Heatmaps<T>,
    value_heatmaps: Heatmaps<T>,
    clocksource: Clocksource,
    poll: Poll,
}

impl<T: Hash + Eq + Send + Clone + Display> Default for Receiver<T> {
    /// create a default `Receiver`
    fn default() -> Self {
        Config::new().build()
    }
}

impl<T: Hash + Eq + Send + Display + Clone> Receiver<T> {
    /// create a new `Receiver` using the defaults
    pub fn new() -> Receiver<T> {
        Default::default()
    }

    /// create a `Receiver` from a tic::Config
    pub fn configured(config: Config<T>) -> Receiver<T> {
        let (data_tx, data_rx) = channel::sync_channel::<Vec<Sample<T>>>(config.capacity);
        let (control_tx, control_rx) = channel::sync_channel::<ControlMessage<T>>(config.capacity);
        let empty_queue = Arc::new(Queue::with_capacity(config.capacity));
        for _ in 0..config.capacity {
            let _ = empty_queue.push(Vec::with_capacity(config.batch_size));
        }

        let clocksource = Clocksource::default();
        let slices = config.duration * config.windows;

        // calculate counter values for start, window, and end times
        let start_time = clocksource.counter();
        let window_duration = (config.duration as f64 * clocksource.frequency()) as u64;
        let window_time = start_time + window_duration;
        let run_duration = config.windows as u64 * window_duration;
        let end_time = start_time + run_duration;

        let max_tau = config.max_tau;

        let poll = Poll::new().unwrap();
        poll.register(
            &data_rx,
            mio::Token(Token::Data as usize),
            Ready::readable(),
            PollOpt::level(),
        ).unwrap();
        poll.register(
            &control_rx,
            mio::Token(Token::Control as usize),
            Ready::readable(),
            PollOpt::level(),
        ).unwrap();

        Receiver {
            window_duration: window_duration,
            window_time: window_time,
            run_duration: run_duration,
            end_time: end_time,
            config: config,
            empty_queue: empty_queue,
            data_tx: data_tx,
            data_rx: data_rx,
            control_tx: control_tx,
            control_rx: control_rx,
            allans: Allans::new(max_tau),
            counters: Counters::new(),
            gauges: Gauges::new(),
            latency_histograms: Histograms::new(),
            value_histograms: Histograms::new(),
            meters: Meters::new(),
            interests: HashSet::new(),
            taus: common::default_taus(),
            percentiles: common::default_percentiles(),
            latency_heatmaps: Heatmaps::new(slices, start_time),
            value_heatmaps: Heatmaps::new(slices, start_time),
            clocksource: clocksource,
            poll: poll,
        }
    }

    /// Create a new Config which can be used to build() a Receiver
    pub fn configure() -> Config<T> {
        Config::default()
    }

    /// returns a clone of the `Sender`
    pub fn get_sender(&self) -> Sender<T> {
        Sender::new(
            Arc::clone(&self.empty_queue),
            self.data_tx.clone(),
            self.control_tx.clone(),
            self.config.batch_size,
        )
    }

    /// returns a clone of the `Clocksource`
    pub fn get_clocksource(&self) -> Clocksource {
        self.clocksource.clone()
    }

    /// creates a `Controller` attached to this `Receiver`
    pub fn get_controller(&self) -> Controller<T> {
        Controller::new(self.control_tx.clone())
    }

    /// register a stat for export
    pub fn add_interest(&mut self, interest: Interest<T>) {
        match interest.clone() {
            Interest::AllanDeviation(key) => {
                self.allans.init(key);
            }
            Interest::Count(key) => {
                self.counters.init(key);
            }
            Interest::Gauge(key) => {
                self.gauges.init(key);
            }
            Interest::LatencyPercentile(key) => {
                self.latency_histograms.init(key);
            }
            Interest::ValuePercentile(key) => {
                self.value_histograms.init(key);
            }
            Interest::LatencyTrace(key, _) |
            Interest::LatencyWaterfall(key, _) => {
                self.latency_heatmaps.init(key);
            }
            Interest::ValueTrace(key, _) |
            Interest::ValueWaterfall(key, _) => {
                self.value_heatmaps.init(key);
            }
        }
        self.interests.insert(interest);
    }

    /// de-register a stat for export
    pub fn remove_interest(&mut self, interest: &Interest<T>) {
        match interest.clone() {
            Interest::AllanDeviation(key) => {
                self.allans.remove(key);
            }
            Interest::Count(key) => {
                self.counters.remove(key);
            }
            Interest::Gauge(key) => {
                self.gauges.remove(key);
            }
            Interest::LatencyPercentile(key) => {
                self.latency_histograms.remove(key);
            }
            Interest::ValuePercentile(key) => {
                self.value_histograms.remove(key);
            }
            Interest::LatencyTrace(key, _) |
            Interest::LatencyWaterfall(key, _) => {
                self.latency_heatmaps.remove(key);
            }
            Interest::ValueTrace(key, _) |
            Interest::ValueWaterfall(key, _) => {
                self.value_heatmaps.remove(key);
            }
        }
        self.interests.remove(interest);
    }

    /// clear the heatmaps
    pub fn clear_heatmaps(&mut self) {
        self.latency_heatmaps.clear();
        self.value_heatmaps.clear();
    }

    /// run the receive loop for one window
    pub fn run_once(&mut self) {
        trace!("run once");

        let window_time = self.window_time;

        loop {
            if self.check_elapsed(window_time) {
                return;
            }

            let mut events = Events::with_capacity(1024);
            self.poll.poll(&mut events, self.config.poll_delay).unwrap();
            for event in events.iter() {
                trace!("got: {} events", events.len());
                let token = event.token().0;
                if token == Token::Data as usize {
                    if let Ok(mut results) = self.data_rx.try_recv() {
                        for result in &results {
                            let t0 = self.clocksource.convert(result.start());
                            let t1 = self.clocksource.convert(result.stop());
                            let dt = t1 - t0;
                            self.allans.record(result.metric(), dt);
                            self.gauges.set(result.metric(), result.value());
                            self.counters.increment_by(result.metric(), result.count());
                            self.latency_histograms.increment(
                                result.metric(),
                                dt as u64,
                            );
                            self.value_histograms.increment(
                                result.metric(),
                                result.count(),
                            );
                            self.latency_heatmaps.increment(
                                result.metric(),
                                t0 as u64,
                                dt as u64,
                            );
                            self.value_heatmaps.increment(
                                result.metric(),
                                t0 as u64,
                                result.count(),
                            );
                        }
                        results.clear();
                        let _ = self.empty_queue.push(results);
                        trace!("finished processing");
                    }
                } else if token == Token::Control as usize {
                    if let Ok(msg) = self.control_rx.try_recv() {
                        match msg {
                            ControlMessage::AddInterest(interest) => {
                                self.add_interest(interest);
                            }
                            ControlMessage::RemoveInterest(interest) => {
                                self.remove_interest(&interest);
                            }
                            ControlMessage::SnapshotMeters(tx) => {
                                let meters = self.clone_meters();
                                tx.send(meters).unwrap();
                            }
                        }
                    }
                }
            }
            trace!("run complete");
        }
    }

    // this function will check if the window is passed
    // if it has, it will refresh the `Meters`
    fn check_elapsed(&mut self, t1: u64) -> bool {
        let tsc = self.clocksource.counter();
        if tsc >= t1 {
            self.meters.clear();
            for interest in &self.interests {
                match *interest {
                    Interest::Count(ref key) => {
                        self.meters.set_count(
                            key.clone(),
                            self.counters.count(key.clone()),
                        );
                    }
                    Interest::Gauge(ref key) => {
                        self.meters.set_value(
                            key.clone(),
                            self.gauges.value(key.clone()),
                        );
                    }
                    Interest::LatencyPercentile(ref key) => {
                        for percentile in self.percentiles.clone() {
                            self.meters.set_latency_percentile(
                                key.clone(),
                                percentile.clone(),
                                self.latency_histograms
                                    .percentile(key.clone(), percentile.1)
                                    .unwrap_or(0),
                            );
                        }
                    }
                    Interest::ValuePercentile(ref key) => {
                        for percentile in self.percentiles.clone() {
                            self.meters.set_value_percentile(
                                key.clone(),
                                percentile.clone(),
                                (self.value_histograms
                                     .percentile(key.clone(), percentile.1)
                                     .unwrap_or(0) as f64 *
                                     self.config.sample_rate) as
                                    u64,
                            );
                        }
                    }
                    Interest::AllanDeviation(ref key) => {
                        for tau in self.taus.clone() {
                            if let Ok(adev) = self.allans.adev(key, tau) {
                                self.meters.set_adev(key.clone(), tau, adev);
                            }
                        }
                    }
                    _ => {}
                }
            }

            self.latency_histograms.clear();
            self.value_histograms.clear();
            self.window_time += self.window_duration;
            return true;
        }
        false
    }

    /// run the receive loop for all windows, output waterfall and traces as requested
    pub fn run(&mut self) {
        let mut window = 0;
        debug!("collection ready");
        'outer: loop {
            'inner: loop {
                self.run_once();
                window += 1;
                if window >= self.config.windows {
                    break 'inner;
                }
            }

            self.save_files();

            if !self.config.service_mode {
                break 'outer;
            } else {
                self.clear_heatmaps();
                self.end_time += self.run_duration;
            }
        }
    }

    /// save all artifacts
    pub fn save_files(&mut self) {
        for interest in self.interests.clone() {
            match interest {
                Interest::LatencyTrace(l, f) => {
                    self.latency_heatmaps.trace(l, f);
                }
                Interest::ValueTrace(l, f) => {
                    self.value_heatmaps.trace(l, f);
                }
                Interest::LatencyWaterfall(l, f) => {
                    self.latency_heatmaps.waterfall(l, f);
                }
                Interest::ValueWaterfall(l, f) => {
                    self.value_heatmaps.waterfall(l, f);
                }
                _ => {}
            }
        }
    }

    /// return a clone of the raw `Meters`
    pub fn clone_meters(&self) -> Meters<T> {
        self.meters.clone()
    }
}

#[cfg(feature = "benchmark")]
#[cfg(test)]
mod benchmark {
    extern crate test;
    use super::*;

    #[bench]
    fn heavy_cycle(b: &mut test::Bencher) {
        let mut receiver = Receiver::<String>::new();
        receiver.add_interest(Interest::Count("test".to_owned()));
        receiver.add_interest(Interest::LatencyPercentile("test".to_owned()));
        receiver.add_interest(Interest::AllanDeviation("test".to_owned()));
        b.iter(|| {
            // full stats evaluation
            receiver.check_elapsed(0);
        });
    }

    #[bench]
    fn cheap_cycle(b: &mut test::Bencher) {
        let mut receiver = Receiver::<String>::new();
        receiver.add_interest(Interest::Count("test".to_owned()));
        receiver.add_interest(Interest::LatencyPercentile("test".to_owned()));
        receiver.add_interest(Interest::AllanDeviation("test".to_owned()));
        b.iter(|| {
            // no stats evaluation just get clock and compare
            receiver.check_elapsed(u64::max_value());
        });
    }
}
