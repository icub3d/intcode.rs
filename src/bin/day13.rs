use std::{
    io::{stdout, Stdout},
    time::Duration,
};

use intcode::{
    ipc::{Channel, ChannelReceiver, ChannelSender},
    process::Process,
    renderer::Monokai,
};

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    text::Text,
    widgets::{block::Title, Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};

use crate::output_event_emitter::OutputEvent;

const INPUT: &str = include_str!("../../inputs/day13");

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long, default_value = "1")]
    part: i32,

    #[arg(short, long)]
    replay: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.part {
        1 => part1().await,
        _ => part2(cli.replay).await?,
    };
    Ok(())
}

async fn part1() {
    // Create our process and start it running.
    let (mut process, _, output) = create_process();
    tokio::spawn(async move {
        match process.run().await {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e),
        }
    });

    // Create our app state and handle output from the process until it's done.
    let mut state = AppState::new();
    let (_, mut output) = output_event_emitter::start(output);
    while let Some(value) = output.recv().await {
        if let OutputEvent::GridUpdate(x, y, tile) = value {
            state.grid[y][x] = tile;
        }
    }

    // Get a count of the block tiles and print it.
    let block_count = state
        .grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|&&tile_id| tile_id == Tile::Block)
        .count();
    println!("p1: {}", block_count);
}

async fn part2(replay: Option<String>) -> Result<()> {
    // Initialize our TUI.
    let mut tui = Tui::new()?;
    tui.init()?;

    // Create our process and start it running.
    let (mut process, mut input, output) = create_process();
    process.set_memory(0, 2);
    tokio::spawn(async move {
        match process.run().await {
            Ok(_) => {}
            Err(e) => eprintln!("{}", e),
        }
    });

    // Setup our event emitters.
    let (output_handle, mut output_events) = output_event_emitter::start(output);
    let (input_handle, mut input_events) = input_event_emitter::start(replay)?;

    // Create our app state.
    let mut app = AppState::new();

    // Main loop to handle input and output. We draw first and then handle each event. We use
    // "biased;" to favor output events because they'll be most important for drawing and there are
    // a lot of them to initialize the game.
    loop {
        tui.draw(&app)?;
        tokio::select! {
            biased;

            // Handle output events.
            evt = output_events.recv() => {
                match evt {
                    Some(OutputEvent::GridUpdate(x, y, tile)) => {
                        app.grid[y][x] = tile;
                    }
                    Some(OutputEvent::Score(score)) => {
                        app.score = score;
                    }
                    None => break,
                }
            }

            // Handle input events.
            evt = input_events.recv() => {
                if let Some(evt) = evt {
                    input.send(evt.into()).await?;
                }
            }

            // Force a redraw occasionally.
            _ = tokio::time::sleep(Duration::from_millis(16)) => {}
        }
    }

    // Signal our tasks to stop and then clean up. Note, if we were concerned about them
    // "finishing" their work, we'd use join instead.
    input_handle.abort();
    output_handle.abort();
    tui.exit()?;
    println!("p2: {}", app.score);
    Ok(())
}

/// A helper function to create the process for day13 and its input and output channels.
fn create_process() -> (Process, ChannelSender, ChannelReceiver) {
    let (_, input_tx, input_rx) = Channel::new(true);
    let (_, output_tx, output_rx) = Channel::new(true);
    let process = Process::new(INPUT, input_rx, output_tx);
    (process, input_tx, output_rx)
}

/// The state of the game.
struct AppState {
    grid: Vec<Vec<Tile>>,
    score: isize,
}

impl AppState {
    fn new() -> Self {
        Self {
            // Somewhat magic numbers we got from part 1.
            grid: vec![vec![Tile::Empty; 44]; 24],
            score: 0,
        }
    }
}

/// A tile represents the state of a cell in the game grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tile {
    Empty,
    Wall,
    Block,
    Paddle,
    Ball,
}

impl From<isize> for Tile {
    fn from(value: isize) -> Self {
        match value {
            0 => Tile::Empty,
            1 => Tile::Wall,
            2 => Tile::Block,
            3 => Tile::Paddle,
            4 => Tile::Ball,
            _ => Tile::Empty,
        }
    }
}

impl From<Tile> for Text<'_> {
    fn from(tile: Tile) -> Self {
        match tile {
            Tile::Empty => " ",
            Tile::Wall => "█",
            Tile::Block => "▒",
            Tile::Paddle => "▀",
            Tile::Ball => "o",
        }
        .into()
    }
}

mod input_event_emitter {
    use anyhow::Result;
    use crossterm::event::{
        Event::Key,
        EventStream,
        KeyCode::{self, Char},
    };
    use futures::{future::FutureExt, StreamExt};
    use tokio::{
        sync::mpsc::{self, Receiver},
        task::JoinHandle,
    };

    /// A representation of the joystick being used.
    pub enum JoyStickEvent {
        Left,
        Right,
        Neutral,
    }

    impl From<char> for JoyStickEvent {
        fn from(c: char) -> Self {
            match c {
                'a' => JoyStickEvent::Left,
                'd' => JoyStickEvent::Right,
                _ => JoyStickEvent::Neutral,
            }
        }
    }

    impl From<KeyCode> for JoyStickEvent {
        fn from(key: KeyCode) -> Self {
            match key {
                Char('a') => JoyStickEvent::Left,
                Char('d') => JoyStickEvent::Right,
                _ => JoyStickEvent::Neutral,
            }
        }
    }

    impl From<JoyStickEvent> for isize {
        fn from(joystick: JoyStickEvent) -> Self {
            match joystick {
                JoyStickEvent::Left => -1,
                JoyStickEvent::Right => 1,
                JoyStickEvent::Neutral => 0,
            }
        }
    }

    /// Start the input event emitter. If a replay is provided, it will be used before listening for
    /// keyboard input.
    pub fn start(
        replay: Option<String>,
    ) -> Result<(JoinHandle<Result<()>>, Receiver<JoyStickEvent>)> {
        // Create a channel to send input events to the main loop.
        let (tx, rx) = mpsc::channel(100);

        // Setup our replay if we have one.
        let replay = if let Some(replay) = replay {
            std::fs::read_to_string(replay)?
        } else {
            String::new()
        };

        // Spawn our handler to send input events to the main loop.
        let handle = tokio::spawn(async move {
            // If we have a replay, send it first.
            for input in replay.trim().chars() {
                tx.send(input.into()).await?;
            }

            // Listen for keyboard input using an event stream
            let mut reader = EventStream::new();
            while let Some(Ok(Key(key))) = reader.next().fuse().await {
                tx.send(key.code.into()).await?;
            }
            Ok(())
        });

        // Return the handle and the receiver.
        Ok((handle, rx))
    }
}

mod output_event_emitter {
    use super::Tile;
    use anyhow::Result;
    use intcode::ipc::ChannelReceiver;
    use tokio::{sync::mpsc::Receiver, task::JoinHandle};

    // A representation of the output events we'll be getting from the process.
    pub enum OutputEvent {
        GridUpdate(usize, usize, Tile),
        Score(isize),
    }

    /// Start processing the given receiver and return an event emitter.
    pub fn start(mut receiver: ChannelReceiver) -> (JoinHandle<Result<()>>, Receiver<OutputEvent>) {
        // Create a channel to send output events to the main loop.
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // This one is much simpler. We can basically just use out handler to track values we
        // receive and then send them to the main loop.
        let handle = tokio::spawn(async move {
            let mut emitter = OutputEventEmitter::new();
            while let Some(value) = receiver.recv().await {
                if let Some(event) = emitter.handle_output(value) {
                    tx.send(event).await?;
                }
            }
            Ok(())
        });
        (handle, rx)
    }

    // We use the state to track where we are in the output stream.
    enum State {
        X,
        Y,
        TileId,
    }

    // The output event emitter.
    struct OutputEventEmitter {
        state: State,
        x: isize,
        y: isize,
    }

    impl OutputEventEmitter {
        fn new() -> Self {
            Self {
                state: State::X,
                x: 0,
                y: 0,
            }
        }

        fn handle_output(&mut self, value: isize) -> Option<OutputEvent> {
            // Given our current state, we update the correct value. When we have all the values,
            // we emit the event.
            match self.state {
                State::X => {
                    self.x = value;
                    self.state = State::Y;
                    None
                }
                State::Y => {
                    self.y = value;
                    self.state = State::TileId;
                    None
                }
                State::TileId => {
                    self.state = State::X;
                    if self.x == -1 && self.y == 0 {
                        self.state = State::X;
                        Some(OutputEvent::Score(value))
                    } else {
                        Some(OutputEvent::GridUpdate(
                            self.x as usize,
                            self.y as usize,
                            value.into(),
                        ))
                    }
                }
            }
        }
    }
}

struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    /// Create a new TUI using crossterm as the backend.
    fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Draw the TUI using the given app state.
    fn draw(&mut self, app: &AppState) -> Result<()> {
        self.terminal.draw(|f| ui(app, f))?;
        Ok(())
    }

    /// Initialize the terminal for the TUI.
    fn init(&mut self) -> Result<()> {
        execute!(stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        // This will help us reset the terminal if we panic.
        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            Self::reset().unwrap();
            panic_hook(panic_info);
        }));

        Ok(())
    }

    fn reset() -> Result<()> {
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    /// Clean up the terminal when we are done.
    fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

fn ui(app: &AppState, f: &mut Frame) {
    // Create a layout for our TUI.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(26),
            Constraint::Length(1),
        ])
        .split(f.size());

    // Draw the title block.
    let title_block = Block::default().style(
        Style::default()
            .fg(Monokai::Background.into())
            .bg(Monokai::Violet.into()),
    );
    let title = Paragraph::new("ARCADE CABINET")
        .block(title_block)
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Draw the status block.
    let status_block = Block::default().style(
        Style::default()
            .fg(Monokai::Background.into())
            .bg(Monokai::Green.into()),
    );
    let status = Paragraph::new(format!(
        "Score: {} | (a) left | (d) right | (s) no move",
        app.score
    ))
    .block(status_block)
    .alignment(Alignment::Left);

    f.render_widget(status, chunks[2]);

    let left = (chunks[1].width - 46) / 2;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(left),
            Constraint::Length(46),
            Constraint::Min(1),
        ])
        .split(chunks[1]);

    let block = Block::default()
        .title(Title::from("BLOCK BREAKER").alignment(Alignment::Center))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Monokai::Orange.into()))
        .border_type(BorderType::Rounded)
        .style(
            Style::default()
                .fg(Monokai::White.into())
                .bg(Monokai::Background.into()),
        );
    let mut rows = vec![];
    for row in app.grid.iter() {
        let row = row.iter().map(|&tile| Cell::from(tile)).collect::<Vec<_>>();
        rows.push(Row::new(row));
    }
    let widths = [Constraint::Length(1); 44];
    let table = Table::new(rows, widths).column_spacing(0).block(block);
    f.render_widget(table, chunks[1]);
}
