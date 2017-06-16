use mpmc::Queue;
use sample::Sample;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Clone)]
/// a Sender is used to push `Sample`s to the `Receiver` it is clonable for sharing between threads
pub struct Sender<T> {
    batch_size: usize,
    buffer: Option<Vec<Sample<T>>>,
    rx_queue: Arc<Queue<Vec<Sample<T>>>>,
    tx_queue: Arc<Queue<Vec<Sample<T>>>>,
}

impl<T: Hash + Eq + Send + Clone> Sender<T> {
    pub fn new(
        tx_queue: Arc<Queue<Vec<Sample<T>>>>,
        rx_queue: Arc<Queue<Vec<Sample<T>>>>,
        batch_size: usize,
    ) -> Sender<T> {
        let buffer = Vec::with_capacity(batch_size);
        Sender {
            batch_size: batch_size,
            buffer: Some(buffer),
            rx_queue: rx_queue,
            tx_queue: tx_queue,
        }
    }

    #[inline]
    /// a function to send a `Sample` to the `Receiver`
    pub fn send(&mut self, sample: Sample<T>) -> Result<(), ()> {
        let mut buffer = self.buffer.take().unwrap();
        buffer.push(sample);
        if buffer.len() >= self.batch_size {
            match self.tx_queue.push(buffer) {
                Ok(_) => {
                    loop {
                        if let Some(b) = self.rx_queue.pop() {
                            self.buffer = Some(b);
                            break;
                        }
                    }
                    Ok(())
                }
                Err(buffer) => {
                    self.buffer = Some(buffer);
                    Err(())
                }
            }
        } else {
            self.buffer = Some(buffer);
            Ok(())
        }
    }

    /// a function to change the batch size of the `Sender`
    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
    }

    #[inline]
    /// mock try_send `Sample` to the `Receiver`
    pub fn try_send(&mut self, sample: Sample<T>) -> Result<(), (Sample<T>)> {
        let mut buffer = self.buffer.take().unwrap();
        if buffer.len() < self.batch_size - 1 {
            buffer.push(sample);
            self.buffer = Some(buffer);
            Ok(())
        } else {
            Err(sample)
        }
    }
}
