use std::io::stdout;

use crate::app::App;
use crate::event::Event;
use crate::renderer::RendererState;

use anyhow::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyEventKind, MouseEvent,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::Backend, Terminal};

/// The tui for the application.
pub struct Tui<B: Backend> {
    terminal: Terminal<B>,
    app: App,
    renderer_state: RendererState,
    pub running: bool,
}

impl<B: Backend> Tui<B> {
    /// Create a new tui with the given terminal and application state.
    pub async fn new(terminal: Terminal<B>, app: App) -> Self {
        let renderer_state = RendererState::new(&app);
        Self {
            terminal,
            app,
            renderer_state,
            running: true,
        }
    }

    /// Setup the terminal for the tui. This should be called before the tui is run.
    pub fn init(&mut self) -> Result<()> {
        execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        enable_raw_mode()?;

        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            Self::reset().expect("failed to reset terminal after panic");
            panic_hook(panic_info);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    fn reset() -> Result<()> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// Draw the tui to the terminal.
    pub fn draw(&mut self) -> Result<()> {
        self.terminal
            .draw(|frame| self.renderer_state.render(&self.app, frame))?;
        Ok(())
    }

    /// Reset the terminal to its original state. This should be called after the tui is done.
    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Handle an event for the tui.
    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key(key).await,
            Event::Mouse(mouse) => self.handle_mouse(mouse).await,
            _ => Ok(()),
        }
    }

    async fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.renderer_state.scroll_up();
            }
            MouseEventKind::ScrollDown => {
                self.renderer_state.scroll_down();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('s') => return self.app.step(self.renderer_state.active_process).await,
            KeyCode::Char('q') => {
                self.running = false;
            }
            KeyCode::Char(c) => {
                if let Some(i) = c.to_digit(10) {
                    let i = i as usize;
                    if i < self.renderer_state.total_processes {
                        self.renderer_state.active_process = i;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
