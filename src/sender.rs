#![allow(deprecated)]

use data::Sample;
use mio::channel;
use mio::channel::TrySendError;
use mpmc::Queue;
use std::hash::Hash;
use std::io;
use std::sync::Arc;

#[derive(Clone)]
/// a Sender is used to push `Sample`s to the `Receiver` it is clonable for sharing between threads
pub struct Sender<T> {
    batch_size: usize,
    buffer: Option<Vec<Sample<T>>>,
    tx_queue: channel::SyncSender<Vec<Sample<T>>>,
    rx_queue: Arc<Queue<Vec<Sample<T>>>>,
}

impl<T: Hash + Eq + Send + Clone> Sender<T> {
    pub fn new(
        rx_queue: Arc<Queue<Vec<Sample<T>>>>,
        tx_queue: channel::SyncSender<Vec<Sample<T>>>,
        batch_size: usize,
    ) -> Sender<T> {
        let buffer = Vec::with_capacity(batch_size);
        Sender {
            batch_size: batch_size,
            buffer: Some(buffer),
            tx_queue: tx_queue,
            rx_queue: rx_queue,
        }
    }

    #[inline]
    /// a function to send a `Sample` to the `Receiver`
    pub fn send(&mut self, sample: Sample<T>) -> Result<(), io::Error> {
        let mut buffer = self.buffer.take().unwrap();
        buffer.push(sample);
        if buffer.len() >= self.batch_size {
            match self.tx_queue.try_send(buffer) {
                Ok(_) => {
                    // try to re-use a buffer, otherwise allocate new
                    if let Some(b) = self.rx_queue.pop() {
                        self.buffer = Some(b);
                    } else {
                        self.buffer = Some(Vec::with_capacity(self.batch_size));
                    }
                    Ok(())
                }
                Err(e) => {
                    match e {
                        TrySendError::Io(e) => {
                            error!("io error: {}", e);
                            Err(e)
                        }
                        TrySendError::Full(buffer) |
                        TrySendError::Disconnected(buffer) => {
                            self.buffer = Some(buffer);
                            Ok(())
                        }
                    }
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
