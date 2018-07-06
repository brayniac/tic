//! a high-performance stats library focused on rates and latencies from timestamped events
//!
//! # Features
//!
//! * high throughput - millions of samples per second
//! * latched histogram - for analyzing the full distribution of sample lengths
//! * heatmaps - to generate distribution traces and waterfalls
//! * meters - to expose readings for client usage
//! * http metrics - simple metrics on http for scraping and monitoring, Prometheus compatible
//! * generic - channel type is generic, and used to label the type of sample
//! * flexible - per channel stats are accessible by registering appropriate `Interest`s
//!
//! # Usage
//!
//! This crate is on [crates.io](https://crates.io/crates/tic) and can be used by adding
//! `tic` to your `Cargo.toml`
//!
//! ```toml
//! [dependencies]
//! tic = "*"
//! ```
//!
//! and to your crate root
//!
//! ```rust
//! extern crate tic;
//! ```
//!
//! # Example: Service Mode
//!
//! This example shows how to use `tic` in a long-running service
//!
//! ```rust
//! use std::fmt;
//! use std::thread;
//! use std::time;
//! use tic::{Interest, Receiver, Sample};
//!
//! // define an enum of stats labels
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! pub enum Metric {
//!     Ok,
//! }
//!
//! // implement the fmt::Display trait
//! impl fmt::Display for Metric {
//!    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//!        match *self {
//!            Metric::Ok => write!(f, "ok"),
//!        }
//!    }
//! }
//!
//! // configure a receiver
//! let mut receiver = Receiver::configure()
//!         .service(true)
//!         .build();
//!
//! // register some interests
//! // record the counts of samples with `Metric::Ok`
//! receiver.add_interest(Interest::Count(Metric::Ok));
//! // record latency percentiles of samples with `Metric::Ok`
//! receiver.add_interest(Interest::LatencyPercentile(Metric::Ok));
//!
//! // get a sender and a clocksource
//! let mut sender = receiver.get_sender();
//! let clocksource = receiver.get_clocksource();
//!
//! // run the receiver in a separate thread
//! thread::spawn(move || { receiver.run(); });
//!
//! // put your application logic here, and increment stats
//! for _ in 0..100 {
//!     let start = clocksource.counter();
//! 	// do some work that takes some time
//!     let stop = clocksource.counter();
//!		sender.send(Sample::new(start, stop, Metric::Ok));
//! }
//!
//! // stats will be available on the http_listen port while main() is running
#![cfg_attr(feature = "cargo-clippy", deny(missing_docs))]
#![cfg_attr(feature = "cargo-clippy", deny(warnings))]
#![cfg_attr(feature = "benchmark", feature(test))]
#[cfg(feature = "benchmark")]
extern crate test;

extern crate allan;
extern crate clocksource;
extern crate fnv;
extern crate heatmap;
extern crate histogram;
#[macro_use]
extern crate log;
extern crate mio;
extern crate mio_extras;
extern crate mpmc;
extern crate tiny_http;
extern crate waterfall;

mod common;
mod config;
mod data;
mod receiver;
mod sender;
mod controller;
mod http;

pub use clocksource::Clocksource;
pub use common::{Interest, Percentile, SECOND};
pub use config::Config;
pub use controller::Controller;
pub use data::{Meters, Sample};
pub use http::HttpReporter;
pub use receiver::Receiver;
pub use sender::Sender;
