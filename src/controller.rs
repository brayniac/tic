use common::ControlMessage;
use data::Meters;
use mio_extras::channel;
use mio_extras::channel::TrySendError;
use std::io;
use std::sync::mpsc;

/// a `Controller` allows interacting with a remote `Receiver`
pub struct Controller<T> {
    control_tx: channel::SyncSender<ControlMessage<T>>,
}

impl<T> Controller<T> {
    pub fn new(control_tx: channel::SyncSender<ControlMessage<T>>) -> Controller<T> {
        Controller { control_tx: control_tx }
    }

    /// takes a snapshot of the current meters by cloning them
    ///
    /// this will block until the `Receiver` responds
    pub fn get_meters(&self) -> Result<Meters<T>, io::Error> {
        let (tx, rx) = mpsc::sync_channel(1);
        let msg = ControlMessage::SnapshotMeters(tx);

        match self.control_tx.try_send(msg) {
            Ok(_) => {
                match rx.recv() {
                    Ok(result) => Ok(result),
                    Err(_) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "failed to receive snapshot",
                    )),
                }
            }
            Err(e) => {
                match e {
                    TrySendError::Io(e) => {
                        error!("io error: {}", e);
                        Err(e)
                    }
                    TrySendError::Full(_) |
                    TrySendError::Disconnected(_) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        "failed to send snapshot command",
                    )),
                }
            }
        }
    }
}
