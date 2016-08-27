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

use tic::{Clocksource, Interest, Receiver, Sample, Sender};

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
        loop {
            let t1 = self.clocksource.counter();
            if let Some(t0) = self.t0 {
                let _ = self.stats.send(Sample::new(t0, t1, Metric::Ok));
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
            let ms = format!("{:.*}",
                             3,
                             ((time::precise_time_ns() % 1_000_000_000) / 1_000_000));
            println!("{}.{} {:<5} [{}] {}",
                     time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(),
                     ms.pad(3, '0', Alignment::Right, true),
                     record.level().to_string(),
                     "tic benchmark",
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

    opts.optopt("p", "producers", "number of producers", "INTEGER");
    opts.optopt("w", "windows", "number of integration windows", "INTEGER");
    opts.optopt("d", "duration", "length of integration window", "INTEGER");
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
    set_log_level(0);
    info!("tic benchmark");

    let windows = matches.opt_str("windows").unwrap_or("60".to_owned()).parse().unwrap();
    let duration = matches.opt_str("duration").unwrap_or("1".to_owned()).parse().unwrap();

    // initialize a Receiver for the benchmark
    let mut receiver = Receiver::configure()
        .windows(windows)
        .duration(duration)
        .capacity(10_000)
        .http_listen("localhost:42024".to_owned())
        .build();

    receiver.add_interest(Interest::Waterfall(Metric::Ok, "ok_waterfall.png".to_owned()));
    receiver.add_interest(Interest::Trace(Metric::Ok, "ok_trace.txt".to_owned()));
    receiver.add_interest(Interest::Count(Metric::Ok));

    let sender = receiver.get_sender();
    let clocksource = receiver.get_clocksource();

    let producers = matches.opt_str("producers").unwrap_or("1".to_owned()).parse().unwrap();

    info!("producers: {}", producers);

    for _ in 0..producers {
        let s = sender.clone();
        let c = clocksource.clone();
        thread::spawn(move || {
            Generator::new(s, c).run();
        });
    }

    let mut total = 0;

    let windows = windows;
    // we run the receiver manually so we can access the Meters
    for _ in 0..windows {
        let t0 = clocksource.time();
        receiver.run_once();
        let t1 = clocksource.time();
        let m = receiver.clone_meters();
        let mut c = 0;
        if let Some(t) = m.get_combined_count() {
            c = *t - total;
            total = *t;
        }
        let r = c as f64 / ((t1 - t0) as f64 / 1_000_000_000.0);
        info!("rate: {} samples per second", r);
    }
    info!("saving files...");
    receiver.save_files();
    info!("saved");
}
