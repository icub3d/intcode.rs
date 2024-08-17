use futures::{
    future::{pending, FutureExt},
    StreamExt,
};

use std::collections::{HashSet, VecDeque};
use std::io::{stdout, Stdout};

use intcode::ipc::{Channel, ChannelReceiver, ChannelSender};
use intcode::process::Process;
use intcode::renderer::ColorScheme;

use anyhow::Result;
use clap::Parser;
use crossterm::event::{Event, EventStream, KeyEvent};
use crossterm::{
    event::KeyCode,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::widgets::{block::Title, Block, BorderType, Borders, List, Paragraph};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
};
use ratatui::{style::Style, text::Text};
use ratatui::{Frame, Terminal};
use serde::Serialize;
use tokio::select;

const INPUT: &str = include_str!("inputs/day17");

/// Point in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: isize,
    y: isize,
}

impl Point {
    fn new(x: isize, y: isize) -> Self {
        Self { x, y }
    }
}

/// Part to run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum Part {
    #[default]
    One,
    Two,
    Gui,
}

/// CLI options using `clap`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Parser)]
struct Cli {
    #[structopt(short, long, default_value_t, value_enum)]
    part: Part,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    match args.part {
        Part::One => part1().await?,
        Part::Two => part2().await?,
        Part::Gui => tui().await?,
    }
    Ok(())
}

/// Run the intcode program and return the input and output channels.
async fn run_program(part: Part) -> (ChannelSender, ChannelReceiver) {
    // The only difference between part 1 and 2 is the memory value at address 0.
    let (_, input_tx, input_rx) = Channel::new(true);
    let (_, output_tx, output_rx) = Channel::new(true);
    tokio::spawn(async move {
        let mut process = Process::new(INPUT, input_rx, output_tx);
        if part == Part::Two || part == Part::Gui {
            process.set_memory(0, 2);
        }
        process.run().await.unwrap();
    });

    (input_tx, output_rx)
}

async fn part1() -> Result<()> {
    // Run our program.
    let (_, mut output_rx) = run_program(Part::One).await;

    // Receive output until the program ends. We'll store the grid in a HashSet.
    let mut x = 0;
    let mut y = 0;
    let mut grid = HashSet::new();
    while let Some(output) = output_rx.recv().await {
        if output == 35 {
            grid.insert(Point::new(x, y));
        }
        if output == 10 {
            y += 1;
            x = 0;
        } else {
            x += 1;
        }
    }

    // Go through the grid and look for intersections. It will be an intersection if the point has
    // a scaffold in all four directions.
    let mut p1 = 0;
    for key in grid.iter() {
        if grid.contains(&Point::new(key.x + 1, key.y))
            && grid.contains(&Point::new(key.x - 1, key.y))
            && grid.contains(&Point::new(key.x, key.y + 1))
            && grid.contains(&Point::new(key.x, key.y - 1))
        {
            p1 += key.x * key.y;
        }
    }
    println!("p1: {}", p1);
    Ok(())
}

async fn part2() -> Result<()> {
    // You can use the GUI to do some manual work to find the path. Mine ended up being like this.
    let input = "A,B,B,A,C,A,C,A,C,B\nL,6,R,12,R,8\nR,8,R,12,L,12\nR,12,L,12,L,4,L,4\nn\n"
        .chars()
        .map(|c| c as isize)
        .collect::<Vec<_>>();

    // Run the program and send the instructions to the program.
    let (mut input_tx, mut output_rx) = run_program(Part::Two).await;
    tokio::spawn(async move {
        for c in input {
            input_tx.send(c).await.unwrap();
        }
    });

    // It will spit out the graph and then eventually the dust collected. We'll look for the dust
    // collected and then print it.
    while let Some(output) = output_rx.recv().await {
        if output > 255 {
            println!("p2: {}", output);
        }
    }
    Ok(())
}

// A helper function to try and write a value if one is ready. If not, it will await a pending,
// which will never return. This is useful for the select macro because we won't always have
// something to write. This way, we can use the same method in the loop.
async fn write_or_wait(mut input: ChannelSender, value: Option<isize>) {
    match value {
        Some(c) => {
            input.send(c).await.unwrap();
        }
        None => {
            pending::<isize>().await;
        }
    }
}

/// The state of the output loop. We'll use these states to determine what to do with the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputState {
    /// We are initializing the grid.
    InitGrid,

    /// We are in the question loop getting questions and sending answers.
    QuestionLoop,

    /// We are redrawing the grid on the right hand side.
    RedrawGrid,

    /// We are done.
    Done,
}

async fn tui() -> Result<()> {
    // Create a new TUI and initialize it.
    let mut tui = Tui::new()?;
    tui.init()?;

    // Start the program running.
    let (input_tx, mut output_rx) = run_program(Part::Gui).await;

    // Variables to keep track of state.
    let mut app = AppState::new();
    let mut reader = EventStream::new();
    let mut inputs = VecDeque::new();

    // Our main loop. We'll run this until we get a quit signal from the event handler.
    loop {
        // We use biased here to make sure outputs are prioritized over other events.
        select! {
            biased;

            // If we get output from the program, we should update our app state.
            val = output_rx.recv(), if app.output_state != OutputState::Done => {
                // Handle any output.
                app.handle_output(val);
            },

            // If we get and even from our event reader, we should update our inputs.
            //
            // From a readability perspective, there is always a question of how much to put in
            // a main even loop. Most of the cases simply call a function. This one does some high
            // level handling. We could also potentially put all of the code in this loop.
            evt = reader.next().fuse() => {
                if let Some(Ok(Event::Key(key))) = evt {
                    if app.handle_input(key, &mut inputs) {
                        break;
                    }
                }
            },

            // If we have something to write, write it out.
            _ = write_or_wait(input_tx.clone(), inputs.pop_front()) => {},

            // If we don't get anything, redraw and start the select again.
            _ = tokio::time::sleep(std::time::Duration::from_millis(16)) => {},
        }

        // After we've handled an event, do a redraw of the TUI.
        tui.draw(&app)?;
    }

    // Clean up the TUI when we are done.
    tui.exit()?;
    Ok(())
}

/// The state of the terminal. The TUI will use this to draw the app.
struct AppState {
    /// The grid of the game.
    grid: Vec<Vec<char>>,

    /// The terminal output.
    terminal: Vec<Vec<char>>,

    /// The current line we are typing.
    line: Vec<char>,

    /// The state of the output loop.
    output_state: OutputState,

    /// The last output we received.
    last_output: isize,

    /// The current x and y position in the grid.
    x: usize,
    y: usize,
}

impl AppState {
    fn new() -> Self {
        Self {
            grid: vec![vec!['.'; 51]; 31],
            line: vec![],
            terminal: vec![vec![]],
            output_state: OutputState::InitGrid,
            last_output: 0,
            x: 0,
            y: 0,
        }
    }

    fn handle_input(&mut self, key: KeyEvent, inputs: &mut VecDeque<isize>) -> bool {
        match (key.code, self.output_state) {
            // If we get a ctrl+q, we'll break out of the loop and exit the program.
            (KeyCode::Char('q'), _) if key.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                false
            }

            // If we are done, ignore all other events.
            (_, OutputState::Done) => true,

            // Handle the backspace in the question loop.
            (KeyCode::Backspace, OutputState::QuestionLoop) => {
                if self.line.pop().is_some() {
                    self.terminal.last_mut().unwrap().pop();
                }
                false
            }

            // Handle the enter key in the question loop. We'll take the line and send
            // it to the program.
            (KeyCode::Enter, OutputState::QuestionLoop) => {
                self.line.iter().for_each(|c| {
                    inputs.push_back(*c as isize);
                });
                inputs.push_back(10);
                self.terminal.push(vec![]);
                self.line.clear();
                false
            }

            // Handle all other characters in the question loop by adding them to the
            // line and the terminal.
            (KeyCode::Char(c), OutputState::QuestionLoop) => {
                self.line.push(c);
                self.terminal.last_mut().unwrap().push(c);
                false
            }
            _ => false,
        }
    }

    fn handle_output(&mut self, output: Option<isize>) {
        // We've received some output. We want to handle it based on the state.
        if let Some(output) = output {
            match (output, self.last_output, self.output_state) {
                // The program is done, print out the dust to the terminal.
                (i, _, _) if i > 255 => {
                    self.terminal.push("dust: ".to_string().chars().collect());
                    self.terminal
                        .last_mut()
                        .unwrap()
                        .extend(output.to_string().chars());
                    self.output_state = OutputState::Done;
                }

                // If we got two newlines while initializing the grid, we move to the question loop.
                (10, 10, OutputState::InitGrid) => {
                    self.output_state = OutputState::QuestionLoop;
                }

                // If we get two newlines in the question loop, we move to redrawing the
                // grid. Reset our x and y back to the top.
                (10, 10, OutputState::QuestionLoop) => {
                    self.terminal.push(vec![]);
                    self.output_state = OutputState::RedrawGrid;
                    self.x = 0;
                    self.y = 0;
                }

                // If we get two newlines and we are redrawing the grid, reset x and y to
                // go back to the top.
                (10, 10, OutputState::RedrawGrid) => {
                    self.x = 0;
                    self.y = 0;
                }

                // If we get a newline while redrawing the grid, move to the next row.
                (10, _, OutputState::RedrawGrid | OutputState::InitGrid) => {
                    self.y += 1;
                    self.x = 0;
                }

                // All other grid drawing states just put the output into the grid.
                (_, _, OutputState::RedrawGrid | OutputState::InitGrid) => {
                    self.grid[self.y][self.x] = output as u8 as char;
                    self.x += 1;
                }

                // For everything else, just put the output into the terminal.
                (_, _, _) => {
                    self.terminal.last_mut().unwrap().push(output as u8 as char);
                }
            }
            self.last_output = output;
        } else {
            // When the output if closed, we'll hit this section and move to done so we
            // don't alter the state anymore.
            self.output_state = OutputState::Done;
        }
    }
}

/// Our TUI App. It looks similar to previous examples.
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

    /// A helper function to reset the terminal.
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

/// Draw the TUI using the given app state in the given frame.
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
            .fg(ColorScheme::Background.into())
            .bg(ColorScheme::Violet.into()),
    );
    let title = Paragraph::new("ASCII TERMINAL")
        .block(title_block)
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Draw the status block.
    let status_block = Block::default().style(
        Style::default()
            .fg(ColorScheme::Background.into())
            .bg(ColorScheme::Green.into()),
    );
    let status = Paragraph::new("(Ctrl+Q) Quit")
        .block(status_block)
        .alignment(Alignment::Left);
    f.render_widget(status, chunks[2]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(30), Constraint::Length(53)])
        .split(chunks[1]);

    // Draw the terminal.
    let terminal_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ColorScheme::Red.into()))
        .border_type(BorderType::Rounded)
        .style(
            Style::default()
                .fg(ColorScheme::White.into())
                .bg(ColorScheme::Background.into()),
        );
    let terminal = List::new(
        app.terminal
            .iter()
            .map(|row| Text::raw(row.iter().collect::<String>()))
            .collect::<Vec<_>>(),
    )
    .block(terminal_block);
    f.render_widget(terminal, chunks[0]);

    // Draw the game grid.
    let block = Block::default()
        .title(Title::from("GRID").alignment(Alignment::Center))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ColorScheme::Orange.into()))
        .border_type(BorderType::Rounded)
        .style(
            Style::default()
                .fg(ColorScheme::White.into())
                .bg(ColorScheme::Background.into()),
        );
    let grid = app
        .grid
        .iter()
        .map(|row| Text::raw(row.iter().collect::<String>()))
        .collect::<Vec<_>>();
    let list = List::new(grid).block(block);
    f.render_widget(list, chunks[1]);
}
