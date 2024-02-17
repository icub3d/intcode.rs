use std::{collections::VecDeque, str::FromStr};

use crate::{app::App, breakpoint::Breakpoints, instruction::Instruction, process};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        block::Title, Block, BorderType, Borders, Cell, Clear, List, Paragraph, Row, Table,
        TableState, Tabs, Wrap,
    },
    Frame,
};

/// The colors palette for the monokai theme.
#[allow(dead_code)]
pub enum Monokai {
    DarkBlack,
    LightBlack,
    Background,
    DarkerGrey,
    DarkGrey,
    Grey,
    LightGrey,
    LighterGrey,
    White,
    Blue,
    Green,
    Violet,
    Orange,
    Red,
    Yellow,
}

impl From<Monokai> for Color {
    fn from(color: Monokai) -> Self {
        match color {
            Monokai::DarkBlack => Color::from_str("#19181a").unwrap(),
            Monokai::LightBlack => Color::from_str("#221f22").unwrap(),
            Monokai::Background => Color::from_str("#2d2a2e").unwrap(),
            Monokai::DarkerGrey => Color::from_str("#403e41").unwrap(),
            Monokai::DarkGrey => Color::from_str("#5b595c").unwrap(),
            Monokai::Grey => Color::from_str("#727072").unwrap(),
            Monokai::LightGrey => Color::from_str("#939293").unwrap(),
            Monokai::LighterGrey => Color::from_str("#c1c0c0").unwrap(),
            Monokai::White => Color::from_str("#fcfcfa").unwrap(),
            Monokai::Blue => Color::from_str("#78dce8").unwrap(),
            Monokai::Green => Color::from_str("#a9dc76").unwrap(),
            Monokai::Violet => Color::from_str("#ab9df2").unwrap(),
            Monokai::Orange => Color::from_str("#fc9867").unwrap(),
            Monokai::Red => Color::from_str("#ff6188").unwrap(),
            Monokai::Yellow => Color::from_str("#ffd866").unwrap(),
        }
    }
}

/// The different states of the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Main,
    BreakpointType,
    BreakpointList,
    BreakpointInstruction,
    BreakpointMemory,
}

/// The state of the renderer.
pub struct RendererState {
    /// The index of the active process.
    pub active_process: usize,

    /// The total number of processes.
    pub total_processes: usize,

    /// The window state of the renderer.
    pub window_state: WindowState,

    /// The breakpoints of the app.
    pub breakpoints: Breakpoints,

    // The index of the chosen instruction for the breakpoint menu.
    pub chosen_instruction: usize,

    // The value of the chosen memory location for the breakpoint menu.
    pub chosen_memory_location: usize,

    memory_rows: Vec<usize>,
    table_states: Vec<TableState>,
}

impl RendererState {
    /// Create a new renderer state with the given app.
    pub fn new(app: &App) -> Self {
        let states = app.states();
        let total_processes = states.len();
        let memory_rows = states
            .iter()
            .map(|state| state.len() / 8)
            .collect::<Vec<_>>();
        let table_states = vec![TableState::default(); total_processes];
        Self {
            active_process: 0,
            total_processes,
            window_state: WindowState::Main,
            breakpoints: Breakpoints::default(),
            chosen_instruction: 0,
            chosen_memory_location: 0,
            memory_rows,
            table_states,
        }
    }

    /// Update the scroll and table states to scroll them "up".
    pub fn scroll_up(&mut self) {
        match self.window_state {
            WindowState::Main => {
                let table_state = &mut self.table_states[self.active_process];
                if table_state.offset() > 0 {
                    table_state.select(Some(table_state.offset() - 1));
                    *table_state.offset_mut() -= 1;
                }
            }
            WindowState::BreakpointInstruction => {
                if self.chosen_instruction > 0 {
                    self.chosen_instruction -= 1;
                }
            }
            _ => {}
        }
    }

    /// Update the scroll and table states to scroll them "down".
    pub fn scroll_down(&mut self) {
        match self.window_state {
            WindowState::Main => {
                let table_state = &mut self.table_states[self.active_process];
                if table_state.offset() < self.memory_rows[self.active_process] - 1 {
                    table_state.select(Some(table_state.offset() + 1));
                    *table_state.offset_mut() += 1;
                }
            }
            WindowState::BreakpointInstruction => {
                if self.chosen_instruction < Instruction::NAMES.len() - 1 {
                    self.chosen_instruction += 1;
                }
            }
            _ => {}
        }
    }

    /// Render the app into the given frame using this state.
    pub fn render(&mut self, app: &App, frame: &mut Frame<'_>) {
        // Create the layout of the different sections of the app.
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(frame.size());

        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(50), Constraint::Max(30)].as_ref())
            .split(rows[2]);

        let sidebar = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(9), Constraint::Max(7), Constraint::Max(10)].as_ref())
            .split(cols[1]);

        // Get all the state information we need.
        let buffers = app.buffers();
        let process_states = app.states();

        Self::draw_header(frame, rows[0]);
        Self::draw_tabs(frame, rows[1], &process_states, self.active_process);
        Self::draw_memory(
            frame,
            cols[0],
            &process_states[self.active_process],
            &mut self.table_states[self.active_process],
        );
        Self::draw_process_state(frame, sidebar[0], &process_states[self.active_process]);
        Self::draw_channels(frame, sidebar[1], &buffers, self.active_process);
        Self::draw_talking_head(frame, sidebar[2]);
        Self::draw_help(frame, rows[3]);

        match self.window_state {
            WindowState::Main => {}
            WindowState::BreakpointList => {
                Self::draw_breakpoint_list(&self.breakpoints, frame);
            }
            WindowState::BreakpointType => {
                Self::draw_breakpoint_type(frame);
            }
            WindowState::BreakpointInstruction => {
                Self::draw_breakpoint_instruction(
                    Instruction::NAMES[self.chosen_instruction],
                    frame,
                );
            }
            WindowState::BreakpointMemory => {
                Self::draw_breakpoint_memory(&self.chosen_memory_location.to_string(), frame);
            }
        }
    }

    fn draw_header(frame: &mut Frame, chunk: Rect) {
        let title_block = Block::default().style(
            Style::default()
                .fg(Monokai::Background.into())
                .bg(Monokai::Violet.into()),
        );

        let title = Paragraph::new("INTCODE COMPUTER")
            .block(title_block)
            .alignment(Alignment::Center);

        frame.render_widget(title, chunk);
    }

    fn draw_tabs(
        frame: &mut Frame<'_>,
        chunk: Rect,
        process_states: &[process::State],
        active_process: usize,
    ) {
        let block = Block::default()
            .title(Title::from("Processes").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Yellow.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let tabs = process_states
            .iter()
            .enumerate()
            .map(|(i, state)| {
                let mut style = Style::default().bg(Monokai::Grey.into());
                if state.halted {
                    style = style.fg(Monokai::Red.into());
                } else if i == active_process {
                    style = style.fg(Monokai::White.into());
                }
                Span::from(format!("<   {}   >", i)).style(style)
            })
            .collect();
        let tabs = Tabs::new(tabs)
            .select(active_process)
            .block(block)
            .style(Style::default().fg(Monokai::DarkerGrey.into()))
            .highlight_style(Style::default().bg(Monokai::Green.into()));

        frame.render_widget(tabs, chunk);
    }

    fn draw_memory(
        frame: &mut Frame<'_>,
        chunk: Rect,
        process_state: &process::State,
        table_state: &mut TableState,
    ) {
        let block = Block::default()
            .title(Title::from("Memory").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Orange.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let (instruction, positions, relatives) = match process_state.next_instruction() {
            Some((instruction, _)) => (
                instruction,
                instruction.position_parameters(),
                instruction.relative_parameters(process_state.relative_base),
            ),
            None => (Instruction::Halt, Vec::new(), Vec::new()),
        };

        // A helper function to draw a chunk of memory and create a row for the table.
        let mut params_left = 0;
        let mut draw_chunk = |start: usize, chunk: &[isize]| {
            let mut row = vec![Cell::from(format!("{:08}", start))
                .style(Style::default().bg(Monokai::DarkerGrey.into()))];
            for (j, v) in chunk.iter().enumerate() {
                let mut style = Style::default().bg(Monokai::Background.into());
                if process_state.instruction_pointer == start + j {
                    style = style.bg(Monokai::Green.into());
                    params_left = instruction.parameter_count();
                } else if params_left > 0 {
                    style = style.bg(Monokai::Red.into());
                    params_left -= 1;
                } else if positions.contains(&(start + j)) || relatives.contains(&(start + j)) {
                    style = style.bg(Monokai::Blue.into());
                }
                row.push(Cell::from(format!("{}", v)).style(style));
            }
            Row::new(row)
        };

        let mut chunks: Vec<_> = process_state
            .memory
            .chunks(8)
            .enumerate()
            .map(|(i, chunk)| draw_chunk(i * 8, chunk))
            .collect();

        // Get the additional memory groups by sorting them and finding the head of each group of
        // 8.
        let mut keys = process_state
            .additional_memory
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort_unstable();
        let mut keys = keys.into_iter().collect::<VecDeque<_>>();
        let mut key_groups = vec![];
        while let Some(head) = keys.pop_front() {
            let mut count = 1;
            while !keys.is_empty() && count < 8 {
                let next = keys.front().unwrap();
                if *next < head + 8 {
                    keys.pop_front();
                    count += 1;
                } else {
                    break;
                }
            }
            key_groups.push(head);
        }

        // Now we can add the additional memory groups to the chunks.
        for key in key_groups {
            let memory = (key..key + 8).map(|i| process_state[i]).collect::<Vec<_>>();
            chunks.push(draw_chunk(key, &memory));
        }

        let widths = [Constraint::Length(10); 9];
        let table = Table::new(chunks, widths)
            .block(block)
            .header(
                Row::new(vec![
                    Cell::from("Location"),
                    Cell::from("+0"),
                    Cell::from("+1"),
                    Cell::from("+2"),
                    Cell::from("+3"),
                    Cell::from("+4"),
                    Cell::from("+5"),
                    Cell::from("+6"),
                    Cell::from("+7"),
                ])
                .style(Style::default().bg(Monokai::DarkerGrey.into())),
            )
            .column_spacing(0);

        frame.render_stateful_widget(table, chunk, table_state);
    }

    fn draw_process_state(frame: &mut Frame<'_>, chunk: Rect, process_state: &process::State) {
        let state_block = Block::default()
            .title(Title::from("State").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Red.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let instruction = match process_state.next_instruction() {
            Some((instruction, _)) => instruction,
            None => Instruction::Halt,
        };

        let states = vec![
            format!("HLT: {:?}", process_state.halted),
            format!("IP:  {:?}", process_state.instruction_pointer),
            format!("RB:  {:?}", process_state.relative_base),
            format!(
                "IO:  [{:?}, {:?}]",
                process_state.last_input, process_state.last_output
            ),
            format!(""),
            format!("{}", instruction),
        ];

        let items: Vec<_> = states.iter().map(Line::raw).collect();
        let list = Paragraph::new(items)
            .block(state_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(list, chunk);
    }

    fn draw_channels(
        frame: &mut Frame<'_>,
        chunk: Rect,
        channels: &[Vec<isize>],
        active_process: usize,
    ) {
        let block = Block::default()
            .title(Title::from("Channels").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Violet.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let channels: Vec<_> = channels
            .iter()
            .enumerate()
            .map(|(i, channel)| {
                let mut style = Style::default().fg(Monokai::LightGrey.into());
                if i == active_process {
                    style = style.fg(Monokai::White.into());
                }
                Span::from(format!("{i}: {:?}", channel)).style(style)
            })
            .collect();

        let list = List::new(channels).block(block);
        frame.render_widget(list, chunk);
    }

    fn draw_talking_head(frame: &mut Frame, chunk: Rect) {
        let block = Block::default()
            .title(Title::from("Talking Head").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Blue.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        frame.render_widget(block, chunk);
    }

    fn draw_help(frame: &mut Frame, chunk: Rect) {
        let block = Block::default().style(
            Style::default()
                .fg(Monokai::Background.into())
                .bg(Monokai::Green.into()),
        );
        let status =
            Paragraph::new("(q)uit | (s)tep | (c)ontinue | (b)reakpoint | list (B)reakpoints | (0-9) select process")
                .block(block)
                .alignment(Alignment::Left);

        frame.render_widget(status, chunk);
    }

    fn draw_breakpoint_type(frame: &mut Frame) {
        let area = Self::centered_rect(25, 30, frame.size());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(Title::from("Breakpoint Type").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Violet.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let types = vec!["(I)nstruction", "(M)emory"];
        let items: Vec<_> = types.into_iter().map(Line::raw).collect();
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }

    fn draw_breakpoint_instruction(selected: &str, frame: &mut Frame) {
        let area = Self::centered_rect(25, 60, frame.size());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(Title::from("Breakpoint Instruction").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Violet.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let instructions: Vec<_> = Instruction::NAMES
            .iter()
            .map(|name| {
                let mut style = Style::default().fg(Monokai::LightGrey.into());
                if name == &selected {
                    style = style.fg(Monokai::White.into());
                }
                Span::from(*name).style(style)
            })
            .collect();

        let list = List::new(instructions).block(block);
        frame.render_widget(list, area);
    }

    fn draw_breakpoint_memory(location: &str, frame: &mut Frame) {
        let area = Self::centered_rect(25, 30, frame.size());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(Title::from("Breakpoint Memory Location").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Violet.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let text = Paragraph::new(location)
            .block(block)
            .style(Style::default().bg(Monokai::DarkerGrey.into()))
            .alignment(Alignment::Center);
        frame.render_widget(text, area);
    }

    fn draw_breakpoint_list(breakpoints: &Breakpoints, frame: &mut Frame) {
        let area = Self::centered_rect(60, 70, frame.size());
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(Title::from("Breakpoints").alignment(Alignment::Center))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Monokai::Violet.into()))
            .border_type(BorderType::Rounded)
            .style(
                Style::default()
                    .fg(Monokai::White.into())
                    .bg(Monokai::Background.into()),
            );

        let items: Vec<_> = breakpoints
            .clone()
            .into_iter()
            .map(|bp| Line::raw(format!("{:?}", bp)))
            .collect();
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        // Cut the given rectangle into three vertical pieces
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        // Then cut the middle vertical piece into three width-wise pieces
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1] // Return the middle chunk
    }
}
