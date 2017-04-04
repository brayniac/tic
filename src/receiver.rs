extern crate clocksource;

use std::fmt::Display;
use std::hash::Hash;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use clocksource::Clocksource;
use mpmc::Queue;
use shuteye;
use tiny_http::{Server, Response, Request};

use config::Config;
use counters::Counters;
use meters::Meters;
use heatmaps::Heatmaps;
use histograms::Histograms;
use sample::Sample;
use sender::Sender;

#[derive(Clone)]
/// an Interest registers a metric for reporting
pub enum Interest<T> {
    Count(T),
    Percentile(T),
    Trace(T, String),
    Waterfall(T, String),
}

#[derive(Clone)]
/// a Percentile is the label plus floating point percentile representation
pub struct Percentile(pub String, pub f64);

/// a `Receiver` processes incoming `Sample`s and generates stats
pub struct Receiver<T> {
    config: Config<T>,
    counters: Counters<T>,
    queue: Arc<Queue<Vec<Sample<T>>>>,
    histograms: Histograms<T>,
    meters: Meters<T>,
    interests: Vec<Interest<T>>,
    percentiles: Vec<Percentile>,
    heatmaps: Heatmaps<T>,
    server: Option<Server>,
    clocksource: Clocksource,
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
        let queue = Arc::new(Queue::<Vec<Sample<T>>>::with_capacity(config.capacity));

        let slices = config.duration * config.windows;

        let listen = config.http_listen.clone();
        let server = start_listener(&listen);

        let clocksource = Clocksource::default();
        let t0 = clocksource.time();

        Receiver {
            config: config,
            counters: Counters::new(),
            queue: queue,
            histograms: Histograms::new(),
            meters: Meters::new(),
            interests: Vec::new(),
            percentiles: default_percentiles(),
            heatmaps: Heatmaps::new(slices, t0),
            server: server,
            clocksource: clocksource,
        }
    }

    /// Create a new Config which can be used to build() a Receiver
    pub fn configure() -> Config<T> {
        Config::default()
    }

    /// returns a clone of the `Sender`
    pub fn get_sender(&self) -> Sender<T> {
        Sender::new(self.queue.clone(), self.config.batch_size)
    }

    /// returns a clone of the `Clocksource`
    pub fn get_clocksource(&self) -> Clocksource {
        self.clocksource.clone()
    }

    /// register a stat for export
    pub fn add_interest(&mut self, interest: Interest<T>) {
        self.interests.push(interest)
    }

    /// clear the heatmaps
    pub fn clear_heatmaps(&mut self) {
        self.heatmaps.clear();
    }

    /// run the receive loop for one window
    pub fn run_once(&mut self) {
        trace!("tic::Receiver::run_once");

        let duration = self.config.duration;

        let t0 = self.clocksource.counter();
        let t1 = t0 + (duration as f64 * self.clocksource.frequency()) as u64;
        let mut t2 = t0 + (0.1 * self.clocksource.frequency()) as u64;

        trace!("tic::Receiver polling");
        'outer: loop {
            if self.clocksource.counter() > t2 {
                self.try_handle_http(&self.server);
                t2 += (0.1 * self.clocksource.frequency()) as u64;
            }

            if !self.check_elapsed(t1) {
                trace!("tic::Reveiver::run_once try handle queue");
                let mut i = 0;
                'inner: loop {
                    if i < self.config.capacity {
                        i += 1;
                    } else {
                        break 'inner;
                    }
                    if let Some(results) = self.queue.pop() {
                        for result in results {
                            let t0 = self.clocksource.convert(result.start());
                            let t1 = self.clocksource.convert(result.stop());
                            let dt = t1 - t0;
                            self.counters.increment(result.metric());
                            self.histograms.increment(result.metric(), dt as u64);
                            self.heatmaps.increment(result.metric(), t0 as u64, dt as u64);
                        }
                    } else {
                        break 'inner;
                    }
                }
            } else {
                trace!("tic::Receiver::run_once complete");
                break 'outer;
            }

            if self.config.poll_delay.is_some() {
                shuteye::sleep(self.config.poll_delay.unwrap());
            }
        }
    }

    fn check_elapsed(&mut self, t1: u64) -> bool {
        let tsc = self.clocksource.counter();
        if tsc >= t1 {
            for interest in self.interests.clone() {
                match interest {
                    Interest::Count(l) => {
                        self.meters.set_count(l.clone(), self.counters.metric_count(l));
                    }
                    Interest::Percentile(l) => {
                        for percentile in self.percentiles.clone() {
                            let v = l.clone();
                            self.meters
                                .set_percentile(v.clone(),
                                                percentile.clone(),
                                                self.histograms
                                                    .metric_percentile(v, percentile.1)
                                                    .unwrap_or(0));
                        }
                    }
                    Interest::Trace(_, _) |
                    Interest::Waterfall(_, _) => {}
                }
            }

            self.meters.set_combined_count(self.counters.total_count());
            for percentile in self.percentiles.clone() {
                self.meters.set_combined_percentile(percentile.clone(),
                                                    self.histograms
                                                        .total_percentile(percentile.1)
                                                        .unwrap_or(0));
            }

            self.histograms.clear();
            return true;
        }
        false
    }

    /// run the receive loop for all windows, output waterfall and traces as requested
    pub fn run(&mut self) {
        let mut window = 0;
        debug!("collection ready");
        loop {
            self.run_once();
            window += 1;
            if window >= self.config.windows {
                break;
            }
        }

        self.save_files();
    }

    /// save all artifacts
    pub fn save_files(&mut self) {
        for interest in self.interests.clone() {
            match interest {
                Interest::Count(_) |
                Interest::Percentile(_) => {}
                Interest::Trace(l, f) => {
                    self.heatmaps.metric_trace(l, f);
                }
                Interest::Waterfall(l, f) => {
                    self.heatmaps.metric_waterfall(l, f);
                }
            }
        }

        self.save_trace();
        self.save_waterfall();
    }

    /// save a heatmap trace file for total heatmap
    pub fn save_trace(&mut self) {
        if let Some(file) = self.config.trace_file.clone() {
            debug!("saving trace file");
            self.heatmaps.total_trace(file);
        }
    }

    /// save a waterfall png for the total heatmap
    pub fn save_waterfall(&mut self) {
        if let Some(file) = self.config.waterfall_file.clone() {
            debug!("stats: saving waterfall render");
            self.heatmaps.total_waterfall(file);
        }
    }

    pub fn clone_meters(&self) -> Meters<T> {
        self.meters.clone()
    }

    // try to handle a http request
    fn try_handle_http(&self, server: &Option<Server>) {
        if let Some(ref s) = *server {
            if let Ok(Some(request)) = s.try_recv() {
                debug!("stats: handle http request");
                self.handle_http(request);
            }
        }
    }

    // actually handle the http request
    fn handle_http(&self, request: Request) {
        let mut output = "".to_owned();

        match request.url() {
            "/histogram" => {
                for bucket in &self.histograms.total {
                    if bucket.count() > 0 {
                        output = output + &format!("{} {}\n", bucket.value(), bucket.count());
                    }
                }
            }
            "/vars" | "/metrics" => {
                for (stat, value) in &self.meters.combined {
                    output = output + &format!("{} {}\n", stat, value);
                }
                for (stat, value) in &self.meters.data {
                    output = output + &format!("{} {}\n", stat, value);
                }
            }
            _ => {
                output = output + "{";
                for (stat, value) in &self.meters.combined {
                    output = output + &format!("\"{}\":{},", stat, value);
                }
                for (stat, value) in &self.meters.data {
                    output = output + &format!("\"{}\":{},", stat, value);
                }
                output.pop();
                output = output + "}";
            }
        }

        let response = Response::from_string(output);
        let _ = request.respond(response);
    }
}

// start the HTTP listener for tic
fn start_listener(listen: &Option<String>) -> Option<Server> {
    if let Some(ref l) = *listen {
        let http_socket = l.to_socket_addrs().unwrap().next().unwrap();

        debug!("stats: starting HTTP listener");
        return Some(Server::http(http_socket).unwrap());
    }
    None
}

// helper function to populate the default `Percentile`s to report
fn default_percentiles() -> Vec<Percentile> {
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
