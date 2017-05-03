extern crate heatmap;
extern crate histogram;

use heatmap::Heatmap;
use histogram::Histogram;
use receiver::Receiver;
use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;
use std::time::Duration;

/// a configuration struct for customizing `Receiver`
#[derive(Clone)]
pub struct Config<T> {
    resource_type: PhantomData<T>,
    pub duration: usize,
    pub windows: usize,
    pub capacity: usize,
    pub batch_size: usize,
    pub service_mode: bool,
    pub poll_delay: Option<Duration>,
    pub http_listen: Option<String>,
    pub trace_file: Option<String>,
    pub waterfall_file: Option<String>,
    pub heatmap_config: heatmap::Config,
    pub histogram_config: histogram::Config,
}

impl<T: Hash + Eq + Send + Display + Clone> Default for Config<T> {
    fn default() -> Config<T> {
        let heatmap_config = Heatmap::configure()
            .slice_duration(1_000_000_000)
            .precision(2);
        let histogram_config = Histogram::configure()
            .max_value(60 * 1_000_000_000)
            .precision(4);
        Config {
            resource_type: PhantomData::<T>,
            duration: 60,
            windows: 60,
            capacity: 256,
            batch_size: 512,
            service_mode: false,
            poll_delay: None,
            http_listen: None,
            trace_file: None,
            waterfall_file: None,
            heatmap_config: heatmap_config,
            histogram_config: histogram_config,
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
        self.heatmap_config
            .num_slices(self.duration * self.windows);
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
        self.heatmap_config
            .num_slices(self.duration * self.windows);
        self
    }

    /// set capacity of the queue: default 256
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
    /// c.capacity(256); // buffer for 256 batches of samples
    /// ```
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// set batch size of the sender: default 512
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
    /// c.batch_size(512); // batch 512 samples in one queue write
    /// ```
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
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

    /// set the poll delay
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// # use std::time::Duration;
    /// let mut c = Receiver::<usize>::configure();
    /// c.poll_delay(Some(Duration::new(0, 100_000)));
    pub fn poll_delay(mut self, delay: Option<Duration>) -> Self {
        self.poll_delay = delay;
        self
    }

    /// set receiver to continuous run mode aka service mode
    ///
    /// # Example
    /// ```
    /// # use tic::Receiver;
    /// let mut c = Receiver::<usize>::configure();
    /// c.service(true);
    pub fn service(mut self, enabled: bool) -> Self {
        self.service_mode = enabled;
        self
    }

    /// Build a new Receiver based on the current configuration
    pub fn build(self) -> Receiver<T> {
        Receiver::configured(self)
    }
}
