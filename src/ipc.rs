use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use tokio::sync::mpsc::{self, Receiver, Sender};

/// The sender end of a channel.
#[derive(Debug, Clone)]
pub struct ChannelSender {
    buffer: Arc<Mutex<VecDeque<isize>>>,
    notifier: Sender<()>,
}

impl ChannelSender {
    fn new(buffer: Arc<Mutex<VecDeque<isize>>>, notifier: Sender<()>) -> Self {
        Self { buffer, notifier }
    }

    /// Send a value to the channel.
    pub async fn send(&mut self, value: isize) -> Result<()> {
        {
            self.buffer.lock().unwrap().push_back(value);
        }
        self.notifier.send(()).await?;
        Ok(())
    }
}

/// The receiver end of a channel.
pub struct ChannelReceiver {
    buffer: Arc<Mutex<VecDeque<isize>>>,
    notifier: Receiver<()>,
    block_on_recv: bool,
}

impl ChannelReceiver {
    fn new(
        buffer: Arc<Mutex<VecDeque<isize>>>,
        notifier: Receiver<()>,
        block_on_recv: bool,
    ) -> Self {
        Self {
            buffer,
            notifier,
            block_on_recv,
        }
    }

    /// Receive a value from the channel. If the channel is empty and the channel was set not to
    /// block, then this will return `None` if the channel is empty.
    pub async fn recv(&mut self) -> Option<isize> {
        match self.block_on_recv {
            true => match self.notifier.recv().await {
                Some(_) => {
                    let value = {
                        let data = self.buffer.lock().unwrap();
                        *data.front().unwrap()
                    };
                    self.buffer.lock().unwrap().pop_front();
                    Some(value)
                }
                None => None,
            },
            false => match self.notifier.try_recv() {
                Ok(_) => {
                    let value = {
                        let data = self.buffer.lock().unwrap();
                        *data.front().unwrap()
                    };
                    self.buffer.lock().unwrap().pop_front();
                    Some(value)
                }
                Err(_) => None,
            },
        }
    }
}

/// An extremely simple implementation of a channel for use with the Intcode computer. We use it
/// mostly so we can view what's being held in the channels buffer.
pub struct Channel {
    buffer: Arc<Mutex<VecDeque<isize>>>,
}

impl Channel {
    /// Create a new channel. If `block_on_recv` is `true`, then the receiver will block until a
    /// value is received. If `false`, then the receiver will return `None` if the channel is empty.
    pub fn new(block_on_recv: bool) -> (Self, ChannelSender, ChannelReceiver) {
        let (notifier_send, notifier_recv) = mpsc::channel(32);
        let buffer = Arc::new(Mutex::new(VecDeque::new()));
        let sender = ChannelSender::new(buffer.clone(), notifier_send);
        let receiver = ChannelReceiver::new(buffer.clone(), notifier_recv, block_on_recv);

        (Self { buffer }, sender, receiver)
    }

    /// Get a copy of this channel's buffer.
    pub fn buffer(&self) -> Vec<isize> {
        self.buffer.lock().unwrap().iter().copied().collect()
    }
}
