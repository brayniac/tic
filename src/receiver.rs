extern crate shuteye;

use std::sync::Arc;
use std::time::Instant;
use std::fmt::Display;
use std::hash::Hash;
use std::net::ToSocketAddrs;

use mpmc::Queue;
use tiny_http::{Server, Response, Request};

use config::Config;
use meters::Meters;
use histograms::Histograms;
use heatmaps::Heatmaps;
use sample::Sample;

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

#[derive(Clone)]
/// a Sender is used to push `Sample`s to the `Receiver` it is clonable for sharing between threads
pub struct Sender<T> {
    queue: Arc<Queue<Sample<T>>>,
}

impl<T: Hash + Eq + Send + Clone> Sender<T> {
    #[inline]
    /// a function to send a `Sample` to the `Receiver`
    pub fn send(&self, sample: Sample<T>) -> Result<(), Sample<T>> {
        self.queue.push(sample)
    }
}

/// a `Receiver` processes incoming `Sample`s and generates stats
pub struct Receiver<T> {
    config: Config<T>,
    queue: Arc<Queue<Sample<T>>>,
    histograms: Histograms<T>,
    meters: Meters<T>,
    interests: Vec<Interest<T>>,
    percentiles: Vec<Percentile>,
    heatmaps: Heatmaps<T>,
    server: Option<Server>,
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
        let queue = Arc::new(Queue::<Sample<T>>::with_capacity(config.capacity));
        let slices = config.duration * config.windows;

        let listen = config.http_listen.clone();
        let server = start_listener(listen);

        Receiver {
            config: config,
            queue: queue,
            histograms: Histograms::new(),
            meters: Meters::new(),
            interests: Vec::new(),
            percentiles: default_percentiles(),
            heatmaps: Heatmaps::new(slices),
            server: server,
        }
    }

    /// Create a new Config which can be used to build() a Receiver
    pub fn configure() -> Config<T> {
        Config::default()
    }

    /// returns a clone of the `Sender`
    pub fn get_sender(&self) -> Sender<T> {
        Sender { queue: self.queue.clone() }
    }

    /// register a stat for export
    pub fn add_interest(&mut self, interest: Interest<T>) {
        self.interests.push(interest)
    }

    /// run the receive loop for one window
    pub fn run_once(&mut self) {
        let duration = self.config.duration;

        let t0 = Instant::now();

        'outer: loop {
            // we process stats and handle elapsed duration
            // more frequently than we handle http requests
            for _ in 0..1000 {
                if let Some(result) = self.queue.pop() {
                    self.histograms.increment(result.metric(), result.duration());
                    self.heatmaps.increment(result.metric(), result.start(), result.duration());
                }

                let t1 = Instant::now();

                if (t1 - t0).as_secs() >= duration as u64 {
                    for interest in self.interests.clone() {
                        match interest {
                            Interest::Count(l) => {
                                self.meters.set_count(l.clone(), self.heatmaps.metric_count(l));
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

                    self.meters.set_combined_count(self.heatmaps.total_count());
                    for percentile in self.percentiles.clone() {
                        self.meters.set_combined_percentile(percentile.clone(),
                                                            self.histograms
                                                                .total_percentile(percentile.1)
                                                                .unwrap_or(0));
                    }

                    self.histograms.clear();
                    break 'outer;
                }
            }

            self.try_handle_http(&self.server);
        }
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
                for (stat, value) in &self.meters.data {
                    output = output + &format!("{} {}\n", stat, value);
                }
            }
            _ => {
                output = output + "{";
                for (stat, value) in &self.meters.data {
                    output = output + &format!("\"{}\":{},", stat, value);
                }
                output = output + "}";
            }
        }

        let response = Response::from_string(output);
        let _ = request.respond(response);
    }
}

// start the HTTP listener for tic
fn start_listener(listen: Option<String>) -> Option<Server> {
    if let Some(ref l) = listen {
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
