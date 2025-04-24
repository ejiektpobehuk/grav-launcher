mod log;
use crate::ui::log::{Entry, Log};
mod list;
use crate::ui::list::ListItem as WListItem;

use log::DownloadStatus;
use ratatui::{
    Frame,
    prelude::*,
    style::{Color, Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use tui_widget_list::{ListBuilder, ListState as WListState, ListView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedLog {
    LauncherLog,
    GameStdout,
    GameStderr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMethod {
    Controller,
    Keyboard,
}

pub struct AppState {
    pub log: Log,
    pub game_stdout: Vec<String>,
    pub game_stderr: Vec<String>,
    pub list_state: WListState,
    pub stdout_state: ListState,
    pub stderr_state: ListState,
    pub focused_log: FocusedLog,
    pub fullscreen_mode: bool,
    pub show_exit_popup: bool,
    pub terminal_focused: bool,
    pub input_method: InputMethod,
}

impl AppState {
    pub fn init() -> Self {
        Self {
            log: Log::new(),
            game_stdout: Vec::new(),
            game_stderr: Vec::new(),
            list_state: WListState::default(),
            stdout_state: ListState::default(),
            stderr_state: ListState::default(),
            focused_log: FocusedLog::LauncherLog,
            fullscreen_mode: false,
            show_exit_popup: false,
            terminal_focused: true,
            input_method: InputMethod::Controller,
        }
    }

    pub const fn next_log(&mut self) {
        self.focused_log = match self.focused_log {
            FocusedLog::LauncherLog => FocusedLog::GameStdout,
            FocusedLog::GameStdout => FocusedLog::GameStderr,
            FocusedLog::GameStderr => FocusedLog::LauncherLog,
        };
    }

    pub const fn prev_log(&mut self) {
        self.focused_log = match self.focused_log {
            FocusedLog::LauncherLog => FocusedLog::GameStderr,
            FocusedLog::GameStdout => FocusedLog::LauncherLog,
            FocusedLog::GameStderr => FocusedLog::GameStdout,
        };
    }

    pub const fn enter_fullscreen(&mut self) {
        self.fullscreen_mode = true;
    }

    pub const fn exit_fullscreen(&mut self) {
        self.fullscreen_mode = false;
    }

    pub const fn show_exit_popup(&mut self) {
        self.show_exit_popup = true;
    }

    pub const fn hide_exit_popup(&mut self) {
        self.show_exit_popup = false;
    }

    pub const fn set_terminal_focus(&mut self, focused: bool) {
        if self.terminal_focused != focused {
            self.terminal_focused = focused;
        }
    }

    pub const fn controller_input_used(&mut self) {
        self.input_method = InputMethod::Controller;
    }

    pub const fn keyboard_input_used(&mut self) {
        self.input_method = InputMethod::Keyboard;
    }
}

pub fn draw(frame: &mut Frame, app_state: &mut AppState) {
    let area = frame.area();

    // Render the main UI frame with title and help text
    render_main_frame(frame, area, app_state);

    if app_state.fullscreen_mode {
        render_fullscreen_view(frame, area, app_state);
    } else {
        render_normal_view(frame, area, app_state);
    }

    // Render exit confirmation popup if needed
    if app_state.show_exit_popup {
        render_exit_popup(frame, area, app_state);
    }
}

fn render_main_frame(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let help_text = get_help_text(app_state);
    let help_line = Line::from(help_text);

    let title = Line::from(" GRAV launcher ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(help_line.right_aligned())
        .border_set(border::THICK);
    frame.render_widget(block, area);
}

fn get_help_text(app_state: &AppState) -> Vec<Span> {
    if app_state.show_exit_popup {
        // Hide normal controls when popup is shown
        vec![]
    } else if app_state.fullscreen_mode {
        if app_state.terminal_focused {
            match app_state.input_method {
                InputMethod::Controller => vec![
                    Span::raw(" Press "),
                    Span::styled("B", Style::default().fg(Color::Red).bold()),
                    Span::raw(" to return to normal view "),
                ],
                InputMethod::Keyboard => vec![
                    Span::raw(" Press "),
                    Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
                    Span::raw(" or "),
                    Span::styled("q", Style::default().fg(Color::Blue).bold()),
                    Span::raw(" to return to normal view "),
                ],
            }
        } else {
            vec![
                Span::raw(" Terminal "),
                Span::styled("NOT FOCUSED", Style::default().fg(Color::Red).bold()),
                Span::raw(" - Controller disabled "),
            ]
        }
    } else if !app_state.terminal_focused {
        vec![
            Span::raw(" Terminal "),
            Span::styled("NOT FOCUSED", Style::default().fg(Color::Red).bold()),
            Span::raw(" - Controller disabled "),
        ]
    } else {
        match app_state.input_method {
            InputMethod::Controller => vec![
                Span::styled(" A", Style::default().fg(Color::Green).bold()),
                Span::raw(" Fullscreen | "),
                Span::styled("B", Style::default().fg(Color::Red).bold()),
                Span::raw(" Exit | "),
                Span::styled("D-Pad", Style::default().fg(Color::Yellow).bold()),
                Span::raw(" Navigate "),
            ],
            InputMethod::Keyboard => vec![
                Span::styled(" Enter", Style::default().fg(Color::Blue).bold()),
                Span::raw(" Fullscreen | "),
                Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
                Span::raw(" Exit | "),
                Span::styled("Arrows", Style::default().fg(Color::Blue).bold()),
                Span::raw(" Navigate "),
            ],
        }
    }
}

fn render_fullscreen_view(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let outer_layout = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area);

    let content_area = Layout::default()
        .margin(2)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(outer_layout[0])[0];

    match app_state.focused_log {
        FocusedLog::LauncherLog => render_fullscreen_launcher_log(frame, content_area, app_state),
        FocusedLog::GameStdout => render_fullscreen_game_stdout(frame, content_area, app_state),
        FocusedLog::GameStderr => render_fullscreen_game_stderr(frame, content_area, app_state),
    }
}

fn render_fullscreen_launcher_log(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let items: Vec<WListItem> = app_state
        .log
        .entries()
        .iter()
        .map(|i| match i {
            Entry::Text(text) => WListItem::new(text),
            Entry::Downloand(download) => match download.status() {
                DownloadStatus::InProgress => {
                    if let Some(total) = download.total() {
                        WListItem::new_gauge(
                            "Downloading: ",
                            (download.current() as f64) / (*total as f64),
                        )
                    } else {
                        WListItem::new(format!("Downloading: {}", download.current()))
                    }
                }
                DownloadStatus::Comple => WListItem::new(Line::from(Span::raw(format!(
                    "Downloaded: {} bytes",
                    download.current()
                )))),
                DownloadStatus::Errored(err) => WListItem::new(format!("Download error: {err}")),
                DownloadStatus::NotStarted => {
                    WListItem::new("Download oopsie: something strange happened")
                }
            },
        })
        .collect();

    let builder = ListBuilder::new(|context| {
        let item = items[context.index].clone();
        let main_axis_size = 1;
        (item, main_axis_size)
    });

    let title = Line::from(" Launcher log (FULLSCREEN) ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(Style::default().fg(Color::Green));

    let list = ListView::new(builder, items.len()).block(block);
    frame.render_stateful_widget(list, area, &mut app_state.list_state);
}

fn render_fullscreen_game_stdout(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let stdouts: Vec<ListItem> = app_state
        .game_stdout
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(i.to_string()));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Game text output (FULLSCREEN) ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(Style::default().fg(Color::Green));

    let stdout = List::new(stdouts).block(block);
    frame.render_stateful_widget(stdout, area, &mut app_state.stdout_state);
}

fn render_fullscreen_game_stderr(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let stderrs: Vec<ListItem> = app_state
        .game_stderr
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(i.to_string()));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Game errors (FULLSCREEN) ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(Style::default().fg(Color::Green));

    let stderr = List::new(stderrs).block(block);
    frame.render_stateful_widget(stderr, area, &mut app_state.stderr_state);
}

fn render_normal_view(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let outer_layout = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area);

    let inner_layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(2)
        .constraints(vec![Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(outer_layout[0]);

    let game_output_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_layout[1]);

    render_launcher_log(frame, inner_layout[0], app_state);
    render_game_stdout(frame, game_output_layout[0], app_state);
    render_game_stderr(frame, game_output_layout[1], app_state);
}

fn render_launcher_log(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let items: Vec<WListItem> = app_state
        .log
        .entries()
        .iter()
        .map(|i| match i {
            Entry::Text(text) => WListItem::new(text),
            Entry::Downloand(download) => match download.status() {
                DownloadStatus::InProgress => {
                    if let Some(total) = download.total() {
                        WListItem::new_gauge(
                            "Downloading: ",
                            (download.current() as f64) / (*total as f64),
                        )
                    } else {
                        WListItem::new(format!("Downloading: {}", download.current()))
                    }
                }
                DownloadStatus::Comple => WListItem::new(Line::from(Span::raw(format!(
                    "Downloaded: {} bytes",
                    download.current()
                )))),
                DownloadStatus::Errored(err) => WListItem::new(format!("Download error: {err}")),
                DownloadStatus::NotStarted => {
                    WListItem::new("Download oopsie: something strange happened")
                }
            },
        })
        .collect();

    let builder = ListBuilder::new(|context| {
        let item = items[context.index].clone();
        let main_axis_size = 1;
        (item, main_axis_size)
    });

    // Define border style based on focus
    let launcher_log_border_style = if app_state.focused_log == FocusedLog::LauncherLog {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let title = Line::from(" Launcher log ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(launcher_log_border_style);

    let list = ListView::new(builder, items.len()).block(block);
    frame.render_stateful_widget(list, area, &mut app_state.list_state);
}

fn render_game_stdout(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let stdouts: Vec<ListItem> = app_state
        .game_stdout
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(i.to_string()));
            ListItem::new(content)
        })
        .collect();

    // Define border style based on focus
    let stdout_border_style = if app_state.focused_log == FocusedLog::GameStdout {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let title = Line::from(" Game text output ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(stdout_border_style);

    let stdout = List::new(stdouts).block(block);
    frame.render_stateful_widget(stdout, area, &mut app_state.stdout_state);
}

fn render_game_stderr(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let stderrs: Vec<ListItem> = app_state
        .game_stderr
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(i.to_string()));
            ListItem::new(content)
        })
        .collect();

    // Define border style based on focus
    let stderr_border_style = if app_state.focused_log == FocusedLog::GameStderr {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    };

    let title = Line::from(" Game errors ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(stderr_border_style);

    let stderr = List::new(stderrs).block(block);
    frame.render_stateful_widget(stderr, area, &mut app_state.stderr_state);
}

fn render_exit_popup(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let popup_area = centered_rect(46, 12, area);

    // Create a popup with text and border
    let popup_block = Block::default()
        .title(" Exit Confirmation ".bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .border_type(BorderType::Rounded);

    // Controls text to display in the popup
    let controls_text = match app_state.input_method {
        InputMethod::Controller => vec![
            Span::styled(" A", Style::default().fg(Color::Green).bold()),
            Span::raw(" - Yes    "),
            Span::styled("B", Style::default().fg(Color::Red).bold()),
            Span::raw(" - No "),
        ],
        InputMethod::Keyboard => vec![
            Span::styled("Enter", Style::default().fg(Color::Blue).bold()),
            Span::raw(" - ("),
            Span::styled("Y", Style::default().fg(Color::Blue).bold()),
            Span::raw(")es    "),
            Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
            Span::raw(" - ("),
            Span::styled("N", Style::default().fg(Color::Blue).bold()),
            Span::raw(")o "),
        ],
    };

    let popup_text = Paragraph::new(vec![
        Line::from("Are you sure you want to exit?"),
        Line::from(""),
        Line::from(controls_text),
    ])
    .block(popup_block)
    .alignment(Alignment::Center)
    .style(Style::default());

    // Render the popup
    frame.render_widget(Clear, popup_area);
    frame.render_widget(popup_text, popup_area);
}

// Helper function to create a centered rectangle of the given size
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
