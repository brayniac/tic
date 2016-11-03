//! tic - time interval counter
//! a high-performance stats library for Rust projects
//!
//! # About
//! tic is using `Histogram`s and `Heatmap`s for storing all `Sample`s. Simply
//! timestamp your start and start events, and pass those along with a metric
//! label to `Sample::new()`; Your metric label could be an Enum, or a String,
//! or Integer, or ...). `Sender` is clonable and can be shared across many
//! threads. A single `Receiver` is capable of processing millions of samples
//! per second. Performance is a top priority
//!
//! # Goals
//! * high-performance stats library for use in Rust projects
//! * export derived metrics as well as histograms
//!
//! # Future work
//! * tests, tests, tests
//! * improve the documentation
//! * make it suitable for long-running applications
//! * stats aggregator
//! * extensive benchmarking
//! * optimization efforts, memory footprint and speed
//!
//!
//! # Usage
//!
//! The pattern is to create a Receiver, aquire a Sender from the Receiver, and
//! send `Sample`s along the sender
//!
//! # Example
//!
//! Checkout benches/src/main.rs

#[macro_use]
extern crate log;
extern crate bytes;
extern crate clocksource;
extern crate mio;
extern crate heatmap;
extern crate histogram;
extern crate waterfall;
extern crate shuteye;
extern crate tiny_http;

mod config;
mod meters;
mod histograms;
mod heatmaps;
mod receiver;
mod sample;

pub use config::Config;
pub use receiver::{Interest, Receiver, Sender, Percentile};
pub use sample::Sample;
pub use meters::Meters;

pub use clocksource::Clocksource;
