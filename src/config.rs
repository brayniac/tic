extern crate heatmap;
extern crate histogram;

use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;

use receiver::Receiver;

use heatmap::Heatmap;
use histogram::Histogram;

/// a configuration struct for customizing `Receiver`
#[derive(Clone)]
pub struct Config<T> {
    pub duration: usize,
    pub windows: usize,
    pub http_listen: Option<String>,
    pub trace_file: Option<String>,
    pub waterfall_file: Option<String>,
    pub heatmap_config: heatmap::Config,
    pub histogram_config: histogram::Config,
    resource_type: PhantomData<T>,
}

impl<T: Hash + Eq + Send + Display + Clone> Default for Config<T> {
    fn default() -> Config<T> {
        let heatmap_config = Heatmap::configure()
            .slice_duration(Duration::new(1, 0))
            .precision(2);
        let histogram_config = Histogram::configure()
            .max_value(60 * 1_000_000_000)
            .precision(4);
        Config {
            duration: 60,
            windows: 60,
            http_listen: None,
            trace_file: None,
            waterfall_file: None,
            heatmap_config: heatmap_config,
            histogram_config: histogram_config,
            resource_type: PhantomData::<T>,
        }
    }
}

impl<T: Hash + Eq + Send + Display + Clone> Config<T> {
    /// create a new tic Config with defaults
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
    /// ```
    pub fn new() -> Config<T> {
        Default::default()
    }

    /// set integration window in seconds: default 60
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
    /// c.duration(60); // set to 60 second integration window
    /// ```
    pub fn duration(mut self, duration: usize) -> Self {
        self.duration = duration;
        self.heatmap_config.num_slices(self.duration * self.windows);
        self
    }

    /// set number of windows to collect: default 60
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
    /// c.windows(60); // collect for 60 x duration and terminate
    /// ```
    pub fn windows(mut self, windows: usize) -> Self {
        self.windows = windows;
        self.heatmap_config.num_slices(self.duration * self.windows);
        self
    }

    /// set the http lister address
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
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
    /// let mut c = Receiver::<usize>::configure();
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
    /// let mut c = Receiver::<usize>::configure();
    /// c.waterfall_file("/tmp/waterfall.png".to_owned()); // waterfall png will render here
    /// ```
    pub fn waterfall_file(mut self, path: String) -> Self {
        self.waterfall_file = Some(path);
        self
    }

    /// Build a new Receiver based on the current configuration
    pub fn build(self) -> Receiver<T> {
        Receiver::configured(self)
    }
}
