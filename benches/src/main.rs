#[macro_use]
extern crate log;

use log::{LogLevel, LogLevelFilter, LogMetadata, LogRecord};

extern crate tic;
extern crate getopts;
extern crate time;

use std::fmt;
use getopts::Options;
use std::env;
use std::thread;

use time::precise_time_ns;

use tic::Receiver as StatsReceiver;
use tic::Sender as StatsSender;
use tic::{Stat};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Ok,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Status::Ok => write!(f, "ok"),
        }
    }
}

struct Generator {
    stats: StatsSender<Status>,
    last: u64,
}

impl Generator {
    fn new(stats: StatsSender<Status>) -> Generator {
        Generator {
            stats: stats,
            last: 0,
        }
    }

    fn run(&mut self) {
        loop {
            let now = precise_time_ns();
            let _ = self.stats.send(Stat::new(self.last, now, Status::Ok));
            self.last = now;
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

    let receiver = StatsReceiver::configure().windows(60).duration(1).build();

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
