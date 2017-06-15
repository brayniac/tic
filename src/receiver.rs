extern crate clocksource;

use allans::Allans;
use clocksource::Clocksource;
use config::Config;
use counters::Counters;
use heatmaps::Heatmaps;
use histograms::Histograms;
use meters::Meters;
use mpmc::Queue;
use sample::Sample;
use sender::Sender;
use shuteye;
use std::fmt::Display;
use std::hash::Hash;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tiny_http::{Request, Response, Server};

#[derive(Clone)]
/// an Interest registers a metric for reporting
pub enum Interest<T> {
    AllanDeviation(T),
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
    window_time: u64,
    window_duration: u64,
    end_time: u64,
    run_duration: u64,
    config: Config<T>,
    rx_queue: Arc<Queue<Vec<Sample<T>>>>,
    tx_queue: Arc<Queue<Vec<Sample<T>>>>,
    allans: Allans<T>,
    counters: Counters<T>,
    histograms: Histograms<T>,
    meters: Meters<T>,
    interests: Vec<Interest<T>>,
    taus: Vec<usize>,
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
        let rx_queue = Arc::new(Queue::<Vec<Sample<T>>>::with_capacity(config.capacity));
        let tx_queue = Arc::new(Queue::<Vec<Sample<T>>>::with_capacity(config.capacity));
        let _ = tx_queue.push(Vec::with_capacity(config.batch_size));

        let slices = config.duration * config.windows;

        let listen = config.http_listen.clone();
        let server = start_listener(&listen);

        let clocksource = Clocksource::default();

        // calculate counter values for start, window, and end times
        let start_time = clocksource.counter();
        let window_duration = (config.duration as f64 * clocksource.frequency()) as u64;
        let window_time = start_time + window_duration;
        let run_duration = config.windows as u64 * window_duration;
        let end_time = start_time + run_duration;

        Receiver {
            window_duration: window_duration,
            window_time: window_time,
            run_duration: run_duration,
            end_time: end_time,
            config: config,
            tx_queue: tx_queue,
            rx_queue: rx_queue,
            allans: Allans::new(),
            counters: Counters::new(),
            histograms: Histograms::new(),
            meters: Meters::new(),
            interests: Vec::new(),
            taus: default_taus(),
            percentiles: default_percentiles(),
            heatmaps: Heatmaps::new(slices, start_time),
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

        Sender::new(self.rx_queue.clone(),
                    self.tx_queue.clone(),
                    self.config.batch_size)
    }

    /// returns a clone of the `Clocksource`
    pub fn get_clocksource(&self) -> Clocksource {
        self.clocksource.clone()
    }

    /// register a stat for export
    pub fn add_interest(&mut self, interest: Interest<T>) {
        match interest.clone() {
            Interest::AllanDeviation(l) => {
                self.allans.init(l);
            }
            Interest::Count(l) => {
                self.counters.init(l);
            }
            Interest::Percentile(l) => {
                self.histograms.init(l);
            }
            Interest::Trace(l, _) |
            Interest::Waterfall(l, _) => {
                self.heatmaps.init(l);
            }
        }
        self.interests.push(interest)
    }

    /// clear the heatmaps
    pub fn clear_heatmaps(&mut self) {
        self.heatmaps.clear();
    }

    /// run the receive loop for one window
    pub fn run_once(&mut self) {
        trace!("tic::Receiver::run_once");

        let window_time = self.window_time;
        let mut http_time = self.clocksource.counter() +
                            (0.1 * self.clocksource.frequency()) as u64;

        trace!("tic::Receiver polling");
        'outer: loop {
            if self.clocksource.counter() > http_time {
                self.try_handle_http(&self.server);
                http_time += (0.1 * self.clocksource.frequency()) as u64;
            }

            if !self.check_elapsed(window_time) {
                trace!("tic::Reveiver::run_once try handle queue");
                let mut i = 0;
                'inner: loop {
                    if i < self.config.capacity {
                        i += 1;
                    } else {
                        break 'inner;
                    }
                    if let Some(mut results) = self.rx_queue.pop() {
                        for result in &results {
                            let t0 = self.clocksource.convert(result.start());
                            let t1 = self.clocksource.convert(result.stop());
                            let dt = t1 - t0;
                            self.allans.record(result.metric(), dt);
                            self.counters.increment_by(result.metric(), result.count());
                            self.histograms.increment(result.metric(), dt as u64);
                            self.heatmaps
                                .increment(result.metric(), t0 as u64, dt as u64);
                        }
                        results.clear();
                        loop {
                            match self.tx_queue.push(results) {
                                Ok(_) => break,
                                Err(r) => results = r,
                            }
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
                        self.meters.set_count(l.clone(), self.counters.count(l));
                    }
                    Interest::Percentile(l) => {
                        for percentile in self.percentiles.clone() {
                            let v = l.clone();
                            self.meters.set_percentile(v.clone(),
                                                       percentile.clone(),
                                                       self.histograms
                                                           .percentile(v, percentile.1)
                                                           .unwrap_or(0));
                        }
                    }
                    Interest::AllanDeviation(key) => {
                        for tau in self.taus.clone() {
                            if let Some(adev) = self.allans.adev(key.clone(), tau) {
                                self.meters.set_adev(key.clone(), tau, adev);
                            }
                        }
                    }
                    Interest::Trace(_, _) |
                    Interest::Waterfall(_, _) => {}
                }
            }

            self.histograms.clear();
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
                self.heatmaps.clear();
                self.end_time += self.run_duration;
            }
        }
    }

    /// save all artifacts
    pub fn save_files(&mut self) {
        for interest in self.interests.clone() {
            match interest {
                Interest::AllanDeviation(_) |
                Interest::Count(_) |
                Interest::Percentile(_) => {}
                Interest::Trace(l, f) => {
                    self.heatmaps.trace(l, f);
                }
                Interest::Waterfall(l, f) => {
                    self.heatmaps.waterfall(l, f);
                }
            }
        }
    }

    pub fn clone_meters(&self) -> Meters<T> {
        self.meters.clone()
    }

    // try to handle a http request
    fn try_handle_http(&self, server: &Option<Server>) {
        if let Some(ref s) = *server {
            if let Ok(Some(request)) = s.try_recv() {
                trace!("stats: handle http request");
                self.handle_http(request);
            }
        }
    }

    // actually handle the http request
    fn handle_http(&self, request: Request) {
        let mut output = "".to_owned();

        match request.url() {
            "/vars" | "/metrics" => {
                for (stat, value) in &self.meters.data {
                    output = output + &format!("{} {}\n", stat, value);
                }
                for (stat, value) in &self.meters.data_float {
                    output = output + &format!("{} {}\n", stat, value);
                }
            }
            _ => {
                output += "{";
                for (stat, value) in &self.meters.data {
                    output = output + &format!("\"{}\":{},", stat, value);
                }
                for (stat, value) in &self.meters.data_float {
                    output = output + &format!("\"{}\":{},", stat, value);
                }
                output.pop();
                output += "}";
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

// helper function to populate the default `Taus`s to report
fn default_taus() -> Vec<usize> {
    let mut t = Vec::new();
    for i in 1..10 {
        t.push(i);
    }
    for i in 1..10 {
        t.push(i * 10);
    }
    for i in 1..11 {
        t.push(i * 100);
    }
    t
}
