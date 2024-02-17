use std::sync::{Arc, Mutex};

use crate::{breakpoint::Breakpoints, ipc::Channel, process::State};

use anyhow::Result;
use tokio::sync::mpsc::Sender;

/// The type of notification to send to the process.
pub enum Notification {
    Step,
    StepUntil(Breakpoints),
}

/// The application state for the tui.
pub struct App {
    channels: Vec<Channel>,
    states: Vec<Arc<Mutex<State>>>,
    notifiers: Vec<Sender<Notification>>,
}

impl App {
    pub fn new(
        channels: Vec<Channel>,
        states: Vec<Arc<Mutex<State>>>,
        notifiers: Vec<Sender<Notification>>,
    ) -> Self {
        Self {
            channels,
            states,
            notifiers,
        }
    }

    /// Send a notification to the process at the given index to take a step.
    pub async fn step(&self, index: usize) -> Result<()> {
        if self.states[index].lock().unwrap().halted {
            return Ok(());
        }
        self.notifiers[index].send(Notification::Step).await?;
        Ok(())
    }

    pub async fn step_until(&self, index: usize, breakpoints: Breakpoints) -> Result<()> {
        if self.states[index].lock().unwrap().halted {
            return Ok(());
        }
        self.notifiers[index]
            .send(Notification::StepUntil(breakpoints))
            .await?;
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
