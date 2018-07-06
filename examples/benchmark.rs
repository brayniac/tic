#![cfg_attr(feature = "cargo-clippy", deny(warnings))]
extern crate getopts;
#[macro_use]
extern crate log;
extern crate tic;
extern crate time;

use getopts::Options;
use log::{LogLevel, LogLevelFilter, LogMetadata, LogRecord};
use std::env;
use std::fmt;
use std::thread;
use tic::{Clocksource, HttpReporter, Interest, Percentile, Receiver, Sample, Sender};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Metric {
    Ok,
    Total,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Metric::Ok => write!(f, "ok"),
            Metric::Total => write!(f, "total"),
        }
    }
}

struct Generator {
    stats: Sender<Metric>,
    t0: Option<u64>,
    clocksource: Clocksource,
    gauge: u64,
}

impl Generator {
    fn new(stats: Sender<Metric>, clocksource: Clocksource) -> Generator {
        Generator {
            stats: stats,
            t0: None,
            clocksource: clocksource,
            gauge: 0,
        }
    }

    fn run(&mut self) {
        loop {
            self.gauge += 1;
            let t1 = self.clocksource.counter();
            if let Some(t0) = self.t0 {
                let _ = self.stats.send(Sample::new(t0, t1, Metric::Ok));
                let _ = self.stats.send(Sample::gauge(self.gauge, Metric::Total));
            }
            self.t0 = Some(t1);
        }
    }
}

pub struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Trace
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!(
                "{} {:<5} [{}] {}",
                time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(),
                record.level().to_string(),
                "tic benchmark",
                record.args()
            );
        }
    }
}

fn set_log_level(level: usize) {
    let log_filter;
    match level {
        0 => {
            log_filter = LogLevelFilter::Info;
        }
        1 => {
            log_filter = LogLevelFilter::Debug;
        }
        _ => {
            log_filter = LogLevelFilter::Trace;
        }
    }
    let _ = log::set_logger(|max_log_level| {
        max_log_level.set(log_filter);
        Box::new(SimpleLogger)
    });
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn opts() -> Options {
    let mut opts = Options::new();

    opts.optopt("p", "producers", "number of producers", "INTEGER");
    opts.optopt("w", "windows", "number of integration windows", "INTEGER");
    opts.optopt("d", "duration", "length of integration window", "INTEGER");
    opts.optopt("c", "capacity", "size of the mpmc queue", "INTEGER");
    opts.optopt(
        "b",
        "batch",
        "batch size of producer writes to queue",
        "INTEGER",
    );
    opts.optflag("h", "help", "print this help menu");

    opts
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let program = &args[0];

    let opts = opts();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            error!("Failed to parse command line args: {}", f);
            return;
        }
    };

    if matches.opt_present("help") {
        print_usage(program, &opts);
        return;
    }
    set_log_level(1);
    info!("tic benchmark");

    let windows = matches
        .opt_str("windows")
        .unwrap_or_else(|| "60".to_owned())
        .parse()
        .unwrap();
    let duration = matches
        .opt_str("duration")
        .unwrap_or_else(|| "1".to_owned())
        .parse()
        .unwrap();
    let capacity = matches
        .opt_str("capacity")
        .unwrap_or_else(|| "10000".to_owned())
        .parse()
        .unwrap();
    let batch = matches
        .opt_str("batch")
        .unwrap_or_else(|| "1".to_owned())
        .parse()
        .unwrap();
    let producers = matches
        .opt_str("producers")
        .unwrap_or_else(|| "1".to_owned())
        .parse()
        .unwrap();

    // initialize a Receiver for the benchmark
    let mut receiver = Receiver::configure()
        .windows(windows)
        .duration(duration)
        .capacity(capacity)
        .batch_size(batch)
        .build();

    let mut http = HttpReporter::new(&receiver, "localhost:42024");
    thread::spawn(move || http.run());

    receiver.add_interest(Interest::LatencyWaterfall(
        Metric::Ok,
        "ok_waterfall.png".to_owned(),
    ));
    receiver.add_interest(Interest::LatencyTrace(
        Metric::Ok,
        "ok_trace.txt".to_owned(),
    ));
    receiver.add_interest(Interest::Count(Metric::Ok));
    receiver.add_interest(Interest::LatencyPercentile(Metric::Ok));
    receiver.add_interest(Interest::Count(Metric::Total));
    receiver.add_interest(Interest::Gauge(Metric::Total));

    let sender = receiver.get_sender();
    let clocksource = receiver.get_clocksource();

    info!("producers: {}", producers);
    info!("batch size: {}", batch);
    info!("capacity: {}", capacity);

    for _ in 0..producers {
        let s = sender.clone();
        let c = clocksource.clone();
        thread::spawn(move || { Generator::new(s, c).run(); });
    }

    let mut total = 0;

    let windows = windows;
    // we run the receiver manually so we can access the Meters
    for _ in 0..windows {
        let t0 = clocksource.time();
        receiver.run_once();
        let t1 = clocksource.time();
        let m = receiver.clone_meters();
        let mut int = 0;
        if let Some(t) = m.count(&Metric::Ok) {
            int += *t;
        }

        if let Some(t) = m.count(&Metric::Total) {
            int += *t;
        }

        let c = int - total;
        total = int;
        let r = c as f64 / ((t1 - t0) as f64 / 1_000_000_000.0);

        info!("rate: {} samples per second", r);
        info!(
            "latency (ns): p50: {} p90: {} p999: {} p9999: {} max: {}",
            m.latency_percentile(&Metric::Ok, Percentile("p50".to_owned(), 50.0))
                .unwrap_or(&0),
            m.latency_percentile(&Metric::Ok, Percentile("p90".to_owned(), 90.0))
                .unwrap_or(&0),
            m.latency_percentile(&Metric::Ok, Percentile("p999".to_owned(), 99.9))
                .unwrap_or(&0),
            m.latency_percentile(&Metric::Ok, Percentile("p9999".to_owned(), 99.99))
                .unwrap_or(&0),
            m.latency_percentile(&Metric::Ok, Percentile("max".to_owned(), 100.0))
                .unwrap_or(&0)
        );
    }

    let m = receiver.clone_meters();
    let mut c = 0;
    if let Some(t) = m.count(&Metric::Ok) {
        c += *t;
    }

    if let Some(t) = m.count(&Metric::Total) {
        c += *t;
    }

    info!("total metrics pushed: {}", c);

    info!("saving files...");
    receiver.save_files();
    info!("saved");
}
