use mpmc::Queue;
use sample::Sample;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Clone)]
/// a Sender is used to push `Sample`s to the `Receiver` it is clonable for sharing between threads
pub struct Sender<T> {
    queue: Arc<Queue<Vec<Sample<T>>>>,
    buffer: Vec<Sample<T>>,
    batch_size: usize,
}

impl<T: Hash + Eq + Send + Clone> Sender<T> {
    pub fn new(queue: Arc<Queue<Vec<Sample<T>>>>, batch_size: usize) -> Sender<T> {
        Sender {
            queue: queue,
            buffer: Vec::new(),
            batch_size: batch_size,
        }
    }

    #[inline]
    /// a function to send a `Sample` to the `Receiver`
    pub fn send(&mut self, sample: Sample<T>) -> Result<(), ()> {
        self.buffer.push(sample);
        if self.buffer.len() >= self.batch_size {
            if self.queue.push(self.buffer.clone()).is_ok() {
                self.buffer.clear();
                return Ok(());
            } else {
                return Err(());
            }
        }
        Ok(())
    }

    /// a function to change the batch size of the `Sender`
    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }

    #[inline]
    /// mock try_send `Sample` to the `Receiver`
    pub fn try_send(&mut self, sample: Sample<T>) -> Result<(), (Sample<T>)> {
        if self.buffer.len() < self.batch_size - 1 {
            self.buffer.push(sample);
            Ok(())
        } else {
            Err(sample)
        }
    }
}
