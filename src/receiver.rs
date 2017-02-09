extern crate clocksource;
extern crate mio;
extern crate shuteye;

use std::fmt::Display;
use std::hash::Hash;
use std::io::{self, Read, Write};
use std::net::ToSocketAddrs;
use std::time::Duration;

use bytes::{Buf, MutBuf};
use clocksource::Clocksource;
use mio::*;
use mio::channel::{SyncSender};
use mio::timer::{Timer};
use tiny_http::{Server, Response, Request};

use config::Config;
use meters::Meters;
use histograms::Histograms;
use heatmaps::Heatmaps;
use sample::Sample;

pub trait TryRead {
    fn try_read_buf<B: MutBuf>(&mut self, buf: &mut B) -> io::Result<Option<usize>>
        where Self : Sized
    {
        // Reads the length of the slice supplied by buf.mut_bytes into the buffer
        // This is not guaranteed to consume an entire datagram or segment.
        // If your protocol is msg based (instead of continuous stream) you should
        // ensure that your buffer is large enough to hold an entire segment (1532 bytes if not jumbo
        // frames)
        let res = self.try_read(unsafe { buf.mut_bytes() });

        if let Ok(Some(cnt)) = res {
            unsafe { buf.advance(cnt); }
        }

        res
    }

    fn try_read(&mut self, buf: &mut [u8]) -> io::Result<Option<usize>>;
}

pub trait TryWrite {
    fn try_write_buf<B: Buf>(&mut self, buf: &mut B) -> io::Result<Option<usize>>
        where Self : Sized
    {
        let res = self.try_write(buf.bytes());

        if let Ok(Some(cnt)) = res {
            buf.advance(cnt);
        }

        res
    }

    fn try_write(&mut self, buf: &[u8]) -> io::Result<Option<usize>>;
}

impl<T: Read> TryRead for T {
    fn try_read(&mut self, dst: &mut [u8]) -> io::Result<Option<usize>> {
        self.read(dst).map_non_block()
    }
}

impl<T: Write> TryWrite for T {
    fn try_write(&mut self, src: &[u8]) -> io::Result<Option<usize>> {
        self.write(src).map_non_block()
    }
}

/// A helper trait to provide the `map_non_block` function on Results.
trait MapNonBlock<T> {
    /// Maps a `Result<T>` to a `Result<Option<T>>` by converting
    /// operation-would-block errors into `Ok(None)`.
    fn map_non_block(self) -> io::Result<Option<T>>;
}

impl<T> MapNonBlock<T> for io::Result<T> {
    fn map_non_block(self) -> io::Result<Option<T>> {
        use std::io::ErrorKind::WouldBlock;

        match self {
            Ok(value) => Ok(Some(value)),
            Err(err) => {
                if let WouldBlock = err.kind() {
                    Ok(None)
                } else {
                    Err(err)
                }
            }
        }
    }
}

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
    queue: SyncSender<Vec<Sample<T>>>,
    buffer: Vec<Sample<T>>,
    batch_size: usize,
}

impl<T: Hash + Eq + Send + Clone> Sender<T> {
    #[inline]
    /// a function to send a `Sample` to the `Receiver`
    pub fn send(&mut self, sample: Sample<T>) -> Result<(), ()> {
        self.buffer.push(sample);
        if self.buffer.len() >= self.batch_size {
            if self.queue.send(self.buffer.clone()).is_ok() {
                self.buffer.clear();
                return Ok(());
            } else {
                return Err(());
            }
        }
        Ok(())
    }

    #[inline]
    /// a function to try to send a `Sample` to the `Receiver`
    pub fn try_send(&mut self, sample: Sample<T>) -> Result<(), ()> {
        self.buffer.push(sample);
        if self.buffer.len() >= self.batch_size {
            if self.queue.try_send(self.buffer.clone()).is_ok() {
                self.buffer.clear();
                return Ok(());
            } else {
                return Err(());
            }
        }
        Ok(())
    }
}

/// a `Receiver` processes incoming `Sample`s and generates stats
pub struct Receiver<T> {
    config: Config<T>,
    tx: SyncSender<Vec<Sample<T>>>,
    rx: mio::channel::Receiver<Vec<Sample<T>>>,
    poll: Poll,
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
        let (tx, rx) = mio::channel::sync_channel(config.capacity);
        let tx = tx;

        let slices = config.duration * config.windows;

        let listen = config.http_listen.clone();
        let server = start_listener(listen);

        let clocksource = Clocksource::default();
        let t0 = clocksource.time();

        let poll = Poll::new().unwrap();

        poll.register(&rx, Token(1), Ready::readable(), PollOpt::level()).unwrap();

        Receiver {
            config: config,
            tx: tx,
            rx: rx,
            poll: poll,
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
        Sender { 
            queue: self.tx.clone(),
            buffer: Vec::new(),
            batch_size: self.config.batch_size,
        }
    }

    /// returns a clone of the `Clocksource`
    pub fn get_clocksource(&self) -> Clocksource {
        self.clocksource.clone()
    }

    /// register a stat for export
    pub fn add_interest(&mut self, interest: Interest<T>) {
        self.interests.push(interest)
    }

    /// run the receive loop for one window
    pub fn run_once(&mut self) {
        let mut events = Events::with_capacity(1024);
        let mut main_timer = Timer::default();
        let mut http_timer = Timer::default();

        self.poll.register(&main_timer, Token(0), Ready::readable(), PollOpt::edge()).unwrap();
        self.poll.register(&http_timer, Token(2), Ready::readable(), PollOpt::edge()).unwrap();
        main_timer.set_timeout(Duration::from_millis(100), ()).unwrap();
        http_timer.set_timeout(Duration::from_millis(100), ()).unwrap();

        let duration = self.config.duration;

        let t0 = self.clocksource.counter();
        let t1 = t0 + (duration as f64 * self.clocksource.frequency()) as u64;

        'outer: loop {
            self.poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();
            for event in events.iter() {
                if event.token() == Token(1) {
                    for _ in 0..(self.config.capacity) {
                        if let Ok(results) = self.rx.try_recv() {
                            for result in results {
                                let t0 = self.clocksource.convert(result.start());
                                let t1 = self.clocksource.convert(result.stop());
                                let dt = t1 - t0;
                                self.histograms.increment(result.metric(), dt as u64);
                                self.heatmaps.increment(result.metric(), t0 as u64, dt as u64);
                            }
                        } else {
                            break;
                        }
                    }
                }
                if event.token() == Token(0) {
                    trace!("check elapsed");
                    if self.check_elapsed(t1) {
                        break 'outer;
                    }
                    main_timer.set_timeout(Duration::from_millis(100), ()).unwrap();
                }
                if event.token() == Token(2) {
                    trace!("serve http");
                    http_timer.set_timeout(Duration::from_millis(100), ()).unwrap();
                    self.try_handle_http(&self.server);
                }
            }
        }
    }

    fn check_elapsed(&mut self, t1: u64) -> bool {
        let tsc = self.clocksource.counter();
        if tsc >= t1 {
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
