#[macro_use]
extern crate log;

extern crate mpmc;
extern crate heatmap;
extern crate histogram;
extern crate waterfall;
extern crate shuteye;
extern crate tiny_http;

mod counters;
mod gauges;

use std::fmt;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Instant;

use heatmap::Heatmap;
use histogram::Histogram;
use tiny_http::{Server, Response, Request};
use waterfall::Waterfall;

use mpmc::Queue;

const ONE_SECOND: u64 = 1_000_000_000;
const ONE_MINUTE: u64 = 60 * ONE_SECOND;

use counters::*;
use gauges::*;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Counter {
    Total,
}

#[allow(unknown_lints, enum_variant_names)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Gauge {
    Percentile50,
    Percentile90,
    Percentile99,
    Percentile999,
    Percentile9999,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Ok,
}

#[derive(Clone)]
pub struct Stat {
    start: u64,
    stop: u64,
    status: Status,
}

impl Stat {
    pub fn new(start: u64, stop: u64, status: Status) -> Stat {
        Stat {
            start: start,
            stop: stop,
            status: status,
        }
    }
}

pub struct Receiver {
    config: Config,
    queue: Arc<Queue<Stat>>,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Status::Ok => write!(f, "ok"),
        }
    }
}

impl fmt::Display for Gauge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Gauge::Percentile50 => write!(f, "p50"),
            Gauge::Percentile90 => write!(f, "p90"),
            Gauge::Percentile99 => write!(f, "p99"),
            Gauge::Percentile999 => write!(f, "p999"),
            Gauge::Percentile9999 => write!(f, "p9999"),
        }
    }
}

impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Counter::Total => write!(f, "total"),
        }
    }
}

fn request_stats(counters: &Counters<Counter>) {
    info!("Requests: Total: {}",
        counters.get(Counter::Total),
        );
}

fn response_stats(counters: &Counters<Counter>) {
    info!("Responses: Ok: {}",
                          counters.get(Counter::Total),
                        );
}

fn pretty_percentile(histogram: &Histogram, percentile: f64) -> String {
    match histogram.percentile(percentile) {
        Ok(v) => format!("{} ns", v),
        Err(e) => e.to_owned(),
    }
}

fn histogram_stats(histogram: &Histogram) {
    info!("Percentiles: p50: {} p90: {} p99: {} p999: {} p9999: {}",
        pretty_percentile(histogram, 50.0),
        pretty_percentile(histogram, 90.0),
        pretty_percentile(histogram, 99.0),
        pretty_percentile(histogram, 99.9),
        pretty_percentile(histogram, 99.99),
    );
}

fn start_listener(listen: Option<String>) -> Option<Server> {
    if let Some(ref l) = listen {
        let http_socket = l.to_socket_addrs().unwrap().next().unwrap();

        debug!("stats: starting HTTP listener");
        return Some(Server::http(http_socket).unwrap());
    }
    None
}

fn try_handle_http(server: &Option<Server>,
                   histogram: &Histogram,
                   gauges: &Gauges<Gauge>,
                   counters: &Counters<Counter>) {
    if let Some(ref s) = *server {
        if let Ok(Some(request)) = s.try_recv() {
            debug!("stats: handle http request");
            handle_http(request, histogram, gauges, counters);
        }
    }
}

fn handle_http(request: Request,
               histogram: &Histogram,
               gauges: &Gauges<Gauge>,
               counters: &Counters<Counter>) {
    let mut output = "".to_owned();

    match request.url() {
        "/histogram" => {
            for bucket in histogram {
                if bucket.count() > 0 {
                    output = output + &format!("{} {}\n", bucket.value(), bucket.count());
                }
            }
        }
        "/vars" => {
            for (stat, value) in &counters.counts {
                output = output + &format!("{}: {}\n", stat, value);
            }
            for (stat, value) in &gauges.data {
                output = output + &format!("{}: {}\n", stat, value);
            }
        }
        _ => {
            output = output + "{";
            for (stat, value) in &counters.counts {
                output = output + &format!("\"{}\":{},", stat, value);
            }
            for (stat, value) in &gauges.data {
                output = output + &format!("\"{}\":{},", stat, value);
            }
            let _ = output.pop();
            output = output + "}";
        }
    }

    let response = Response::from_string(output);
    let _ = request.respond(response);
}

#[derive(Clone)]
pub struct Sender {
    queue: Arc<Queue<Stat>>,
}


impl Sender {
    pub fn send(&self, stat: Stat) -> Result<(), Stat> {
        self.queue.push(stat)
    }
}

/// a configuration struct for customizing `Receiver`
pub struct Config {
    duration: usize,
    windows: usize,
    http_listen: Option<String>,
    trace_file: Option<String>,
    waterfall_file: Option<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            duration: 60,
            windows: 60,
            http_listen: None,
            trace_file: None,
            waterfall_file: None,
        }
    }
}

impl Config {
    /// create a new tic Config with defaults
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::configure();
    /// ```
    pub fn new() -> Config {
        Default::default()
    }

    /// set integration window in seconds: default 60
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::configure();
    /// c.duration(60); // set to 60 second integration window
    /// ```
    pub fn duration(mut self, duration: usize) -> Self {
        self.duration = duration;
        self
    }

    /// set number of windows to collect: default 60
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::configure();
    /// c.windows(60); // collect for 60 x duration and terminate
    /// ```
    pub fn windows(mut self, windows: usize) -> Self {
        self.windows = windows;
        self
    }

    /// set the http lister address
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::configure();
    /// c.http_listen("0.0.0.0:42024".to_owned()); // listen on port 42024 on all interfaces
    /// ```
    pub fn http_listen(mut self, address: String) -> Self {
        self.http_listen = Some(address);
        self
    }

    /// set the heatmap trace file
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::configure();
    /// c.trace_file("/tmp/heatmap.trace".to_owned()); // heatmap trace will write here
    /// ```
    pub fn trace_file(mut self, path: String) -> Self {
        self.trace_file = Some(path);
        self
    }

    /// set the heatmap trace file
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::configure();
    /// c.waterfall_file("/tmp/waterfall.png".to_owned()); // waterfall png will render here
    /// ```
    pub fn waterfall_file(mut self, path: String) -> Self {
        self.waterfall_file = Some(path);
        self
    }

    /// Build a new Receiver based on the current configuration
    pub fn build(self) -> Receiver {
        Receiver::configured(self)
    }
}

impl Default for Receiver {
    fn default() -> Self {
        Config::new().build()
    }
}

impl Receiver {
    pub fn new() -> Receiver {
        Default::default()
    }

    pub fn configured(config: Config) -> Receiver {
        let queue = Arc::new(Queue::<Stat>::with_capacity(8));
        Receiver {
            config: config,
            queue: queue,
        }
    }

    pub fn configure() -> Config {
        Config::default()
    }

    pub fn get_sender(&self) -> Sender {
        Sender { queue: self.queue.clone() }
    }

    pub fn run(&self) {
        let duration = self.config.duration;
        let windows = self.config.windows;
        let trace = self.config.trace_file.clone();
        let waterfall = self.config.waterfall_file.clone();
        let listen = self.config.http_listen.clone();

        debug!("stats: initialize datastructures");
        let mut histogram = Histogram::configure()
            .precision(4)
            .max_value(ONE_MINUTE)
            .build()
            .unwrap();
        let mut http_histogram = histogram.clone();

        let mut heatmap = Heatmap::configure()
            .precision(2)
            .max_value(ONE_SECOND)
            .slice_duration(ONE_SECOND)
            .num_slices((duration * windows))
            .build()
            .unwrap();

        let mut t0 = Instant::now();
        let mut window_counters = Counters::new();
        let mut global_counters = Counters::new();
        let mut gauges = Gauges::new();
        let mut window = 1;

        let server = start_listener(listen);

        debug!("stats: collection ready");
        loop {
            if let Some(result) = self.queue.pop() {
                window_counters.increment(Counter::Total);
                match result.status {
                    Status::Ok => {
                        let _ = histogram.increment(result.stop - result.start);
                        let _ = heatmap.increment(result.start, result.stop - result.start);
                    }
                }
            } //TODO: add a stat here?

            try_handle_http(&server, &http_histogram, &gauges, &global_counters);

            let t1 = Instant::now();

            if (t1 - t0).as_secs() >= duration as u64 {
                let rate = window_counters.rate(Counter::Total);
                info!("-----");
                info!("Window: {}", window);
                request_stats(&window_counters);
                response_stats(&window_counters);
                info!("Rate: {:.*} rps", 2, rate);
                info!("Latency: min: {} ns max: {} ns",
                        histogram.minimum().unwrap_or(0),
                        histogram.maximum().unwrap_or(0),
                    );
                histogram_stats(&histogram);

                // set gauges to match window stats
                gauges.set(Gauge::Percentile50, histogram.percentile(50.0).unwrap_or(0));
                gauges.set(Gauge::Percentile90, histogram.percentile(90.0).unwrap_or(0));
                gauges.set(Gauge::Percentile99, histogram.percentile(99.0).unwrap_or(0));
                gauges.set(Gauge::Percentile999,
                           histogram.percentile(99.9).unwrap_or(0));
                gauges.set(Gauge::Percentile9999,
                           histogram.percentile(99.99).unwrap_or(0));

                // increment global counters
                for c in [Counter::Total].into_iter() {
                    global_counters.add(c.clone(), window_counters.get(c.clone()));
                }

                http_histogram = histogram.clone();

                // clear the window stats
                histogram.clear();
                window_counters.clear();

                window += 1;
                t0 = t1;
                if window >= windows {
                    if let Some(file) = trace {
                        debug!("stats: saving trace file");
                        heatmap.save(file);
                    }
                    if let Some(file) = waterfall {
                        debug!("stats: saving waterfall render");
                        let mut waterfall = Waterfall { heatmap: heatmap };
                        waterfall.render_png(file);
                    }
                    break;
                }
            }
        }
    }
}
