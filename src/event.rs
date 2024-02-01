use std::time::Duration;

use anyhow::{anyhow, Result};
use crossterm::event::{EventStream, KeyEvent, MouseEvent};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc::{self, UnboundedReceiver};

/// An event that can be handled by the event handler.
#[derive(Debug, Copy, Clone)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
}

/// An event handler that can be used to handle events for the tui.
pub struct EventHandler {
    rx: UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate. We'll render the tui at the tick rate
    /// unless there is an event to handle earlier.
    pub fn new(tick_rate: Duration) -> Self {
        // Create the channel to communicate.
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        // TODO: pass handler to close on exit?
        // Spawn the handler.
        let _handler = tokio::spawn(async move {
            // Create the event stream and ticker
            let tx = _tx;
            let mut reader = EventStream::new();
            let mut tick = tokio::time::interval(tick_rate);

            loop {
                // Setup our futures.
                let tick_delay = tick.tick();
                let event = reader.next().fuse();

                // Select on our futures and send the corresponding event.
                tokio::select! {
                    _ = tick_delay => {
                        tx.send(Event::Tick).unwrap();
                    }
                    event = event => {
                        if let Some(Ok(event)) = event {
                            match event {
                                crossterm::event::Event::Key(key) => {
                                    tx.send(Event::Key(key)).unwrap();
                                }
                                crossterm::event::Event::Mouse(mouse) => {
                                    tx.send(Event::Mouse(mouse)).unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        });
        Self { rx }
    }

    /// Get the next event from the event handler.
    pub async fn next(&mut self) -> Result<Event> {
        self.rx.recv().await.ok_or(anyhow!("no event"))
    }
}
