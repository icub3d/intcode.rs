use std::sync::{Arc, Mutex};

use crate::{
    ipc::Channel,
    process::{Process, State},
};

use anyhow::Result;
use tokio::sync::mpsc::{self, Sender};

/// The application state for the tui.
pub struct App {
    channels: Vec<Channel>,
    states: Vec<Arc<Mutex<State>>>,
    notifiers: Vec<Sender<()>>,
}

impl App {
    /// Create a new application state.
    /// TODO: this will likely need to change in future days to work with different features of the
    /// program.
    pub async fn new(input: &'static str) -> Result<Self> {
        // This works like day 7 but we want to allow the tui to control the process and see into
        // the state of the process.
        let permutation = [0, 1, 2, 3, 4];
        let (channel, mut sender, mut receiver) = Channel::new(false);
        let first = sender.clone();

        // This is the information the app will track.
        let mut channels = vec![channel];
        let mut states = Vec::new();
        let mut notifiers = Vec::new();

        for (i, p) in permutation.iter().enumerate() {
            // Send messages and create the new channel.
            sender.send(*p as isize + 5).await?;
            if i == 0 {
                sender.send(0).await?;
            }
            let (channel, new_sender, new_receiver) = Channel::new(false);
            let new_sender = if i == 4 { first.clone() } else { new_sender };
            if i != 4 {
                channels.push(channel);
            }

            // Create our process and save our state
            let mut process = Process::new(input, receiver, new_sender.clone());
            let state = Arc::new(Mutex::new(process.state()));
            states.push(state.clone());

            // Create our notifier.
            let (notifier, mut notifier_receiver) = mpsc::channel::<()>(32);
            notifiers.push(notifier);

            // Spawn the process loop.
            tokio::spawn(async move {
                // Wait for a notification to take a step.
                while notifier_receiver.recv().await.is_some() {
                    // We don't want to take a step if we are done.
                    if process.state().halted {
                        break;
                    }

                    // Take a step and update the state.
                    process.step().await.unwrap();
                    *state.lock().unwrap() = process.state();
                }
            });
            (sender, receiver) = (new_sender, new_receiver);
        }

        Ok(Self {
            channels,
            states,
            notifiers,
        })
    }

    /// Send a notification to the process at the given index to take a step.
    pub async fn step(&self, index: usize) -> Result<()> {
        if self.states[index].lock().unwrap().halted {
            return Ok(());
        }
        self.notifiers[index].send(()).await?;
        Ok(())
    }

    /// Get the buffers for the channels.
    pub fn buffers(&self) -> Vec<Vec<isize>> {
        self.channels.iter().map(|c| c.buffer()).collect()
    }

    /// Get the states of the processes.
    pub fn states(&self) -> Vec<State> {
        self.states
            .iter()
            .map(|s| s.lock().unwrap().clone())
            .collect()
    }
}
