#[macro_use]
extern crate log;
extern crate shuteye;
extern crate rand;

use log::{LogLevel, LogLevelFilter, LogMetadata, LogRecord};

extern crate tic;
extern crate getopts;
extern crate time;
extern crate pad;

use pad::{PadStr, Alignment};
use std::fmt;
use getopts::Options;
use std::env;
use std::thread;
use std::time::Duration;

use tic::{Clocksource, Interest, Percentile, Receiver, Sample, Sender};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Metric {
    Ok,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Metric::Ok => write!(f, "ok"),
        }
    }
}

struct Generator {
    stats: Sender<Metric>,
    t0: Option<u64>,
    clocksource: Clocksource,
}

impl Generator {
    fn new(stats: Sender<Metric>, clocksource: Clocksource) -> Generator {
        Generator {
            stats: stats,
            t0: None,
            clocksource: clocksource,
        }
    }

    fn run(&mut self) {
        let mut t1 = time::precise_time_ns() + 1_000_000_000;
        loop {
            let t = time::precise_time_ns();
            if t > t1 {
                let t2 = self.clocksource.time();
                self.stats.send(Sample::new(t, t2, Metric::Ok));
                t1 += 1_000_000_000;
            }
            let t1 = self.clocksource.counter();
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
            println!("{} {:<5} [{}] {}",
                     time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(),
                     record.level().to_string(),
                     "allanping",
                     record.args());
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

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn opts() -> Options {
    let mut opts = Options::new();

    opts.optopt("w", "windows", "number of integration windows", "INTEGER");
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
        print_usage(program, opts);
        return;
    }
    set_log_level(1);
    info!("tic benchmark");

    let windows = matches
        .opt_str("windows")
        .unwrap_or_else(|| "60".to_owned())
        .parse()
        .unwrap();
    let duration = 1;
    let capacity = 10_000;
    let batch = 1;

    // initialize a Receiver for the benchmark
    let mut receiver = Receiver::configure()
        .windows(windows)
        .duration(duration)
        .capacity(capacity)
        .batch_size(batch)
        .service(true)
        .http_listen("localhost:42024".to_owned())
        .build();

    receiver.add_interest(Interest::Waterfall(Metric::Ok, "ok_waterfall.png".to_owned()));
    receiver.add_interest(Interest::Trace(Metric::Ok, "ok_trace.txt".to_owned()));
    receiver.add_interest(Interest::Count(Metric::Ok));
    receiver.add_interest(Interest::Percentile(Metric::Ok));
    receiver.add_interest(Interest::AllanDeviation(Metric::Ok));

    let sender = receiver.get_sender();
    let clocksource = receiver.get_clocksource();

    let s = sender.clone();
    let mut c = clocksource.clone();

    // delay and recal the clocksource to get a more accurate frequency estimate
    thread::sleep(Duration::new(60, 0));
    c.recalibrate();
    thread::spawn(move || { Generator::new(s, c).run(); });

    let mut total = 0;

    let windows = windows;
    // we run the receiver manually so we can access the Meters
    for _ in 0..windows {
        let t0 = clocksource.time();
        receiver.run_once();
        let t1 = clocksource.time();
        let m = receiver.clone_meters();
        let mut c = 0;
        if let Some(t) = m.count(&Metric::Ok) {
            c = *t - total;
            total = *t;
        }
        let r = c as f64 / ((t1 - t0) as f64 / 1_000_000_000.0);

        info!("rate: {} samples per second", r);
        info!("latency (ns): p50: {} p90: {} p999: {} p9999: {} max: {}",
              m.percentile(&Metric::Ok, Percentile("p50".to_owned(), 50.0))
                  .unwrap_or(&0),
              m.percentile(&Metric::Ok, Percentile("p90".to_owned(), 90.0))
                  .unwrap_or(&0),
              m.percentile(&Metric::Ok, Percentile("p999".to_owned(), 99.9))
                  .unwrap_or(&0),
              m.percentile(&Metric::Ok, Percentile("p9999".to_owned(), 99.99))
                  .unwrap_or(&0),
              m.percentile(&Metric::Ok, Percentile("max".to_owned(), 100.0))
                  .unwrap_or(&0));
        info!("ADEV:    t=1: {}", m.adev(Metric::Ok, 1).unwrap_or(&0.0));
        info!("ADEV:    t=2: {}", m.adev(Metric::Ok, 2).unwrap_or(&0.0));
        info!("ADEV:    t=5: {}", m.adev(Metric::Ok, 5).unwrap_or(&0.0));
        info!("ADEV:   t=10: {}", m.adev(Metric::Ok, 10).unwrap_or(&0.0));
        info!("ADEV:   t=20: {}", m.adev(Metric::Ok, 20).unwrap_or(&0.0));
        info!("ADEV:   t=50: {}", m.adev(Metric::Ok, 50).unwrap_or(&0.0));
        info!("ADEV:  t=100: {}", m.adev(Metric::Ok, 100).unwrap_or(&0.0));
        info!("ADEV:  t=200: {}", m.adev(Metric::Ok, 200).unwrap_or(&0.0));
        info!("ADEV:  t=500: {}", m.adev(Metric::Ok, 500).unwrap_or(&0.0));
        info!("ADEV: t=1000: {}", m.adev(Metric::Ok, 1000).unwrap_or(&0.0));
    }
    info!("saving files...");
    receiver.save_files();
    info!("saved");
}
