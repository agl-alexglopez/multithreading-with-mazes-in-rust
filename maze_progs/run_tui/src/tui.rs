use crate::args;
use crate::run;
use crate::tables;

use crossbeam_channel::{self, unbounded};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, KeyEvent};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use rand::distributions::Bernoulli;
use rand::prelude::Distribution;
use rand::{seq::SliceRandom, thread_rng};
use ratatui::prelude::Alignment;
use ratatui::widgets::ScrollDirection;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::Wrap;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Color, CrosstermBackend, Modifier},
    style::Style,
    widgets::{Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation},
};
use solvers::solve;
use tui_textarea::{Input, Key, TextArea};

use std::{
    thread,
    time::{Duration, Instant},
};

pub static PLACEHOLDER: &'static str = "Start Typing to Enter Command";

pub type CtEvent = crossterm::event::Event;

#[derive(Debug)]
pub enum Pack {
    Tick,
    Press(KeyEvent),
    Resize(u16, u16),
}

#[derive(Debug)]
pub struct EventHandler {
    pub sender: crossbeam_channel::Sender<Pack>,
    pub receiver: crossbeam_channel::Receiver<Pack>,
    pub handler: thread::JoinHandle<()>,
}

pub struct Tui {
    pub terminal: CrosstermTerminal,
    pub events: EventHandler,
    pub instructions_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Dimension {
    pub rows: i32,
    pub cols: i32,
    pub offset: maze::Offset,
}

pub type Frame<'a> = ratatui::Frame<'a, CrosstermBackend<std::io::Stderr>>;
pub type CrosstermTerminal = ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stderr>>;
pub type Err = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, Err>;

impl Tui {
    /// Constructs a new instance of [`Tui`].
    pub fn new(terminal: CrosstermTerminal, events: EventHandler) -> Self {
        Self {
            terminal,
            events,
            instructions_scroll_state: ScrollbarState::default(),
            vertical_scroll: 0,
        }
    }

    /// Initializes the terminal interface.
    ///
    /// It enables the raw mode and sets terminal properties.
    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;

        // Define a custom panic hook to reset the terminal properties.
        // This way, you won't have your terminal messed up if an unexpected error happens.
        let panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset the terminal");
            panic_hook(panic);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn inner_dimensions(&mut self) -> Dimension {
        let f = self.terminal.get_frame();
        let overall_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(90), Constraint::Percentage(10)])
            .split(f.size());
        let upper_portion = overall_layout[0];
        Dimension {
            rows: (upper_portion.height - 1) as i32,
            cols: (upper_portion.width - 1) as i32,
            offset: maze::Offset {
                add_rows: upper_portion.y as i32,
                add_cols: upper_portion.x as i32,
            },
        }
    }

    /// Resets the terminal interface.
    ///
    /// This function is also used for the panic hook to revert
    /// the terminal properties if unexpected errors occur.
    fn reset() -> Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// Exits the terminal interface.
    ///
    /// It disables the raw mode and reverts back the terminal properties.
    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn home(&mut self, cmd: &mut TextArea) -> Result<()> {
        self.terminal.draw(|frame| {
            ui_home(
                cmd,
                &mut self.vertical_scroll,
                &mut self.instructions_scroll_state,
                frame,
            )
        })?;
        Ok(())
    }

    pub fn background_maze(&mut self) -> Result<()> {
        self.terminal.draw(|frame| ui_bg_maze(frame))?;
        Ok(())
    }

    pub fn scroll(&mut self, dir: ScrollDirection) {
        match dir {
            ScrollDirection::Forward => {
                self.vertical_scroll = self.vertical_scroll.saturating_add(1);
                self.instructions_scroll_state = self
                    .instructions_scroll_state
                    .position(self.vertical_scroll as u16);
            }
            ScrollDirection::Backward => {
                self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
                self.instructions_scroll_state = self
                    .instructions_scroll_state
                    .position(self.vertical_scroll as u16);
            }
        }
    }

    pub fn error_popup(&mut self, msg: String) -> Result<()> {
        self.background_maze()?;
        self.terminal.draw(|f| ui_err(&msg, f))?;
        'reading_message: loop {
            match self.events.next()? {
                Pack::Press(_) | Pack::Resize(_, _) => {
                    break 'reading_message;
                }
                _ => {}
            }
        }
        self.background_maze()?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        let mut cmd_prompt = TextArea::default();
        cmd_prompt.set_cursor_line_style(Style::default());
        cmd_prompt.set_placeholder_text(PLACEHOLDER);
        cmd_prompt.set_placeholder_style(Style::default().fg(Color::LightYellow));
        let text_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));
        cmd_prompt.set_block(text_block);
        cmd_prompt.set_alignment(Alignment::Center);
        self.background_maze()?;
        'render: loop {
            self.home(&mut cmd_prompt)?;
            match self.events.next()? {
                Pack::Resize(_, _) => {
                    self.background_maze()?;
                }
                Pack::Press(ev) => {
                    match ev.into() {
                        Input { key: Key::Esc, .. } => break 'render,
                        Input { key: Key::Down, .. } => self.scroll(ScrollDirection::Forward),
                        Input { key: Key::Up, .. } => self.scroll(ScrollDirection::Backward),
                        Input {
                            key: Key::Enter, ..
                        } => {
                            run::run_command(&cmd_prompt.lines()[0], self)?;
                            //run::rand_with_channels(self)?;
                            self.terminal.clear()?;
                            self.background_maze()?;
                        }
                        input => {
                            // TextArea::input returns if the input modified its text
                            let _ = cmd_prompt.input(input);
                        }
                    }
                }
                Pack::Tick => {}
            }
        }
        Ok(())
    }
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`].
    pub fn new(tick_rate: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate);
        let (sender, receiver) = unbounded();
        let handler = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(Instant::now().elapsed())
                        .unwrap_or(tick_rate);

                    if event::poll(timeout).expect("no events available") {
                        match event::read().expect("unable to read event") {
                            CtEvent::Key(e) => {
                                if e.kind == event::KeyEventKind::Press {
                                    sender.send(Pack::Press(e)).expect("couldn't send.");
                                }
                            }
                            CtEvent::Resize(w, h) => {
                                sender.send(Pack::Resize(w, h)).expect("could not send.");
                            }
                            _ => {}
                        }
                    }
                    // Ticks are important for some submodule channel communications.
                    if last_tick.elapsed() >= tick_rate {
                        sender.send(Pack::Tick).expect("failed to send tick event");
                        last_tick = Instant::now();
                    }
                }
            })
        };
        Self {
            sender,
            receiver,
            handler,
        }
    }

    /// Receive the next event from the handler thread.
    ///
    /// This function will always block the current thread if
    /// there is no data available and it's possible for more data to be sent.
    pub fn next(&self) -> Result<Pack> {
        Ok(self.receiver.recv()?)
    }
}

fn ui_bg_maze(f: &mut Frame<'_>) {
    let frame_block = Block::default().padding(Padding::new(1, 1, 1, 1));
    let mut background_maze = args::MazeRunner::new();
    let mut rng = thread_rng();
    background_maze.args.style = match tables::WALL_STYLES.choose(&mut rng) {
        Some(&style) => style.1,
        None => print::maze_panic!("Styles not found."),
    };
    let modification_probability = Bernoulli::new(0.2);
    background_maze.modify = None;
    if modification_probability
        .expect("Bernoulli innefective")
        .sample(&mut rng)
    {
        background_maze.modify = match tables::MODIFICATIONS.choose(&mut rng) {
            Some(&m) => Some(m.1),
            None => print::maze_panic!("Modification table empty."),
        }
    }
    let mut rng = thread_rng();
    background_maze.build.0 = match &tables::BUILDERS.choose(&mut rng) {
        Some(b) => b.1 .0,
        None => print::maze_panic!("Builder table empty!"),
    };
    let inner = frame_block.inner(f.size());
    background_maze.args.odd_rows = inner.height as i32;
    background_maze.args.odd_cols = inner.width as i32;
    background_maze.args.offset = maze::Offset {
        add_rows: inner.y as i32,
        add_cols: inner.x as i32,
    };
    let mut bg_maze = maze::Maze::new(background_maze.args);
    background_maze.build.0(&mut bg_maze);
    match background_maze.modify {
        Some(m) => m.0(&mut bg_maze),
        _ => {}
    }
    let monitor = solve::Solver::new(bg_maze);
    background_maze.solve.0(monitor.clone());
    match monitor.clone().lock() {
        Ok(lk) => solve::print_paths(&lk.maze),
        Err(_) => print::maze_panic!("Home screen broke."),
    }
}

fn ui_home(
    cmd: &mut TextArea,
    scroll: &mut usize,
    scroll_state: &mut ScrollbarState,
    f: &mut Frame<'_>,
) {
    let overall_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(f.size());
    let frame_block = Block::default().padding(Padding::new(1, 1, 1, 1));
    let popup_layout_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - 80) / 2),
            Constraint::Percentage(80),
            Constraint::Percentage((100 - 80) / 2),
        ])
        .split(overall_layout[0]);
    let popup_layout_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - 50) / 2),
            Constraint::Min(70),
            Constraint::Percentage((100 - 50) / 2),
        ])
        .split(popup_layout_v[1])[1];
    let popup_instructions = Paragraph::new(INSTRUCTIONS)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(Color::Yellow))
                .style(Style::default().bg(Color::Black)),
        )
        .alignment(Alignment::Center)
        .scroll((*scroll as u16, 0));
    f.render_widget(frame_block, overall_layout[0]);
    f.render_widget(popup_instructions, popup_layout_h);
    // I can scroll but the scrollbar does not appear?
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("█")
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        popup_layout_v[0],
        scroll_state,
    );
    let text_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - 15) / 2),
            Constraint::Min(3),
            Constraint::Percentage((100 - 15) / 2),
        ])
        .split(overall_layout[1]);
    let text_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - 50) / 2),
            Constraint::Min(70),
            Constraint::Percentage((100 - 50) / 2),
        ])
        .split(text_v[1])[1];
    let tb = cmd.widget();
    f.render_widget(tb, text_h);
}

fn ui_err(msg: &str, f: &mut Frame<'_>) {
    let popup_layout_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - 15) / 2),
            Constraint::Min(4),
            Constraint::Percentage((100 - 15) / 2),
        ])
        .split(f.size());
    let popup_layout_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - 30) / 2),
            Constraint::Percentage(30),
            Constraint::Percentage((100 - 30) / 2),
        ])
        .split(popup_layout_v[1])[1];
    let popup_instructions = Paragraph::new(msg)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(Color::Black).bg(Color::Red))
                .style(
                    Style::default()
                        .bg(Color::Black)
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .alignment(Alignment::Center);
    f.render_widget(popup_instructions, popup_layout_h);
}

static INSTRUCTIONS: &'static str = "
███╗   ███╗ █████╗ ███████╗███████╗    ████████╗██╗   ██╗██╗
████╗ ████║██╔══██╗╚══███╔╝██╔════╝    ╚══██╔══╝██║   ██║██║
██╔████╔██║███████║  ███╔╝ █████╗         ██║   ██║   ██║██║
██║╚██╔╝██║██╔══██║ ███╔╝  ██╔══╝         ██║   ██║   ██║██║
██║ ╚═╝ ██║██║  ██║███████╗███████╗       ██║   ╚██████╔╝██║
╚═╝     ╚═╝╚═╝  ╚═╝╚══════╝╚══════╝       ╚═╝    ╚═════╝ ╚═╝

- Use flags, followed by arguments, in any order
- Press <ENTER> to confirm your flag choices.

(scroll with <↓>/<↑>, exit with <ESC>)

BUILDER FLAG[-b] Set maze building algorithm.
    [rdfs] - Randomized Depth First Search.
    [kruskal] - Randomized Kruskal's algorithm.
    [prim] - Randomized Prim's algorithm.
    [eller] - Randomized Eller's algorithm.
    [wilson] - Loop-Erased Random Path Carver.
    [wilso]n-walls - Loop-Erased Random Wall Adder.
    [fractal] - Randomized recursive subdivision.
    [grid] - A random grid pattern.
    [arena] - Open floor with no walls.

MODIFICATION FLAG[-m] Add shortcuts to the maze.
    [cross]- Add crossroads through the center.
    [x]- Add an x of crossing paths through center.

SOLVER FLAG[-s] Set maze solving algorithm.
    [dfs-hunt] - Depth First Search
    [dfs-gather] - Depth First Search
    [dfs-corners] - Depth First Search
    [floodfs-hunt] - Depth First Search
    [floodfs-gather] - Depth First Search
    [floodfs-corners] - Depth First Search
    [rdfs-hunt] - Randomized Depth First Search
    [rdfs-gather] - Randomized Depth First Search
    [rdfs-corners] - Randomized Depth First Search
    [bfs-hunt] - Breadth First Search
    [bfs-gather] - Breadth First Search
    [bfs-corners] - Breadth First Search
    [dark[algorithm]-[game]] - A mystery...

WALL FLAG[-w] Set the wall style for the maze.
    [sharp] - The default straight lines.
    [round] - Rounded corners.
    [doubles] - Sharp double lines.
    [bold] - Thicker straight lines.
    [contrast] - Full block width and height walls.
    [spikes] - Connected lines with spikes.

SOLVER ANIMATION FLAG[-sa] Watch the maze solution.
    [1-7] - Speed increases with number.

BUILDER ANIMATION FLAG[-ba] Watch the maze build.
    [1-7] - Speed increases with number.

Cancel any animation by pressing any key.
Zoom out/in with <Ctrl-[-]>/<Ctrl-[+]>
If any flags are omitted, defaults are used.
An empty command line will create a random maze.

EXAMPLES:

-b rdfs -s bfs-hunt
-s bfs-gather -b prim
-s bfs-corners -d round -b fractal
-s dfs-hunt -ba 4 -sa 5 -b wilson-walls -m x

Enjoy!
";
