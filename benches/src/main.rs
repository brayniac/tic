#[macro_use]
extern crate log;
extern crate shuteye;

use log::{LogLevel, LogLevelFilter, LogMetadata, LogRecord};

extern crate tic;
extern crate getopts;
extern crate time;

use std::fmt;
use std::time::Instant;
use getopts::Options;
use std::env;
use std::thread;

use tic::{Interest, Receiver, Sample, Sender};

//use shuteye::*;

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
    t0: Option<Instant>,
}

impl Generator {
    fn new(stats: Sender<Metric>) -> Generator {
        Generator {
            stats: stats,
            t0: None,
        }
    }

    fn run(&mut self) {

        // let ts = Timespec::from_nano(60_000_000_000).unwrap();
        // shuteye::sleep(ts);
        loop {
            let t1 = Instant::now();
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
            println!("{} {:<5} [{}] {}",
                     time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(),
                     record.level().to_string(),
                     "benchmark",
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

    let mut receiver = Receiver::configure()
        .windows(15)
        .duration(60)
        .http_listen("localhost:42024".to_owned())
        .build();

    receiver.add_interest(Interest::Waterfall(Metric::Ok, "ok_waterfall.png".to_owned()));
    receiver.add_interest(Interest::Count(Metric::Ok));

    let sender = receiver.get_sender();

    let producers = matches.opt_str("producers").unwrap_or("1".to_owned()).parse().unwrap();

    info!("producers: {}", producers);

    for _ in 0..producers {
        let s = sender.clone();
        thread::spawn(move || {
            Generator::new(s).run();
        });
    }

    receiver.run();
}
