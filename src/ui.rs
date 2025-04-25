pub mod log;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Normal,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitPopupState {
    Hidden,
    Visible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalFocus {
    Focused,
    Unfocused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    NotRequested,
    Requested,
}

pub struct AppState {
    pub log: Log,
    pub game_stdout: Vec<String>,
    pub game_stderr: Vec<String>,
    pub list_state: WListState,
    pub stdout_state: ListState,
    pub stderr_state: ListState,
    pub focused_log: FocusedLog,
    pub display_mode: DisplayMode,
    pub exit_popup: ExitPopupState,
    pub terminal_focus: TerminalFocus,
    pub input_method: InputMethod,
    pub launcher_update_available: Option<String>,
    pub update_status: UpdateStatus,
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
            display_mode: DisplayMode::Normal,
            exit_popup: ExitPopupState::Hidden,
            terminal_focus: TerminalFocus::Focused,
            input_method: InputMethod::Controller,
            launcher_update_available: None,
            update_status: UpdateStatus::NotRequested,
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
        self.display_mode = DisplayMode::Fullscreen;
    }

    pub const fn exit_fullscreen(&mut self) {
        self.display_mode = DisplayMode::Normal;
    }

    pub const fn show_exit_popup(&mut self) {
        self.exit_popup = ExitPopupState::Visible;
    }

    pub const fn hide_exit_popup(&mut self) {
        self.exit_popup = ExitPopupState::Hidden;
    }

    pub fn set_terminal_focus(&mut self, focused: bool) {
        if (focused && self.terminal_focus == TerminalFocus::Unfocused)
            || (!focused && self.terminal_focus == TerminalFocus::Focused)
        {
            self.terminal_focus = if focused {
                TerminalFocus::Focused
            } else {
                TerminalFocus::Unfocused
            };
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

    if app_state.display_mode == DisplayMode::Fullscreen {
        render_fullscreen_view(frame, area, app_state);
    } else {
        render_normal_view(frame, area, app_state);
    }

    // Render exit confirmation popup if needed
    if app_state.exit_popup == ExitPopupState::Visible {
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
    if app_state.exit_popup == ExitPopupState::Visible {
        // Hide normal controls when popup is shown
        vec![]
    } else if app_state.display_mode == DisplayMode::Fullscreen {
        match app_state.input_method {
            InputMethod::Controller => vec![
                Span::styled(" B", Style::default().fg(Color::Red).bold()),
                Span::raw(" Back "),
            ],
            InputMethod::Keyboard => vec![
                Span::styled(" Esc", Style::default().fg(Color::Blue).bold()),
                Span::raw(" Back "),
            ],
        }
    } else if app_state.terminal_focus == TerminalFocus::Unfocused {
        vec![
            Span::raw(" Terminal "),
            Span::styled("NOT FOCUSED", Style::default().fg(Color::Red).bold()),
            Span::raw(" - Controller disabled "),
        ]
    } else {
        // Add controls based on input method
        match app_state.input_method {
            InputMethod::Controller => {
                let mut controls = vec![
                    Span::styled(" A", Style::default().fg(Color::Green).bold()),
                    Span::raw(" Fullscreen | "),
                    Span::styled("B", Style::default().fg(Color::Red).bold()),
                    Span::raw(" Exit"),
                ];

                // Only show update hint if an update is available and not already in progress
                if app_state.launcher_update_available.is_some()
                    && app_state.update_status == UpdateStatus::NotRequested
                {
                    controls.push(Span::raw(" | "));
                    controls.push(Span::styled("Y", Style::default().fg(Color::Yellow).bold()));
                    controls.push(Span::raw(" Update"));
                }

                controls.push(Span::raw(" | "));
                controls.push(Span::styled(
                    "D-Pad",
                    Style::default().fg(Color::Yellow).bold(),
                ));
                controls.push(Span::raw(" Navigate "));

                controls
            }
            InputMethod::Keyboard => {
                let mut controls = vec![
                    Span::styled(" Enter", Style::default().fg(Color::Blue).bold()),
                    Span::raw(" Fullscreen | "),
                    Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
                    Span::raw(" Exit"),
                ];

                // Only show update hint if an update is available and not already in progress
                if app_state.launcher_update_available.is_some()
                    && app_state.update_status == UpdateStatus::NotRequested
                {
                    controls.push(Span::raw(" | "));
                    controls.push(Span::styled("u", Style::default().fg(Color::Yellow).bold()));
                    controls.push(Span::raw(" Update"));
                }

                controls.push(Span::raw(" | "));
                controls.push(Span::styled(
                    "Arrows",
                    Style::default().fg(Color::Blue).bold(),
                ));
                controls.push(Span::raw(" Navigate "));

                controls
            }
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

// Helper function to format file sizes in a human-readable way
fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2}GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2}MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2}KB", size as f64 / KB as f64)
    } else {
        format!("{size}B")
    }
}

fn render_fullscreen_launcher_log(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    // Build the list of items for the log
    let mut items: Vec<WListItem> = Vec::new();

    // We'll use entries() from Log which now includes everything
    items.extend(app_state.log.entries().iter().map(|i| match i {
        Entry::Text(text) => WListItem::new(text),
        Entry::Downloand(download) => WListItem::new(format!(
            "Download: {}",
            format_file_size(download.current())
        )),
        Entry::LauncherUpdate(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        format!(
                            "Launcher update: {} / {} ",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::new(format!(
                        "Launcher update: {}",
                        format_file_size(download.current())
                    ))
                }
            }
            DownloadStatus::Comple => WListItem::new(format!(
                "Launcher update: {} Downloaded. Restart needed.",
                format_file_size(download.current())
            )),
            DownloadStatus::Errored(err) => WListItem::new(format!("Launcher update error: {err}")),
        },
        Entry::GameDownload(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        format!(
                            "Downloading game: {} / {} ",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::new(format!(
                        "Downloading game: {}",
                        format_file_size(download.current())
                    ))
                }
            }
            DownloadStatus::Comple => WListItem::new(format!(
                "Game downloaded: {}",
                format_file_size(download.current())
            )),
            DownloadStatus::Errored(err) => WListItem::new(format!("Game download error: {err}")),
        },
    }));

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

    let title = Line::from(" Launcher log (FULLSCREEN) ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK)
        .border_style(launcher_log_border_style);

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
    // Build the list of items for the log
    let mut items: Vec<WListItem> = Vec::new();

    // We'll use entries() from Log which now includes everything
    items.extend(app_state.log.entries().iter().map(|i| match i {
        Entry::Text(text) => WListItem::new(text),
        Entry::Downloand(download) => WListItem::new(format!(
            "Download: {}",
            format_file_size(download.current())
        )),
        Entry::LauncherUpdate(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        format!(
                            "Launcher update: {} / {} ",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::new(format!(
                        "Launcher update: {}",
                        format_file_size(download.current())
                    ))
                }
            }
            DownloadStatus::Comple => WListItem::new(format!(
                "Launcher update: {} Downloaded. Restart needed.",
                format_file_size(download.current())
            )),
            DownloadStatus::Errored(err) => WListItem::new(format!("Launcher update error: {err}")),
        },
        Entry::GameDownload(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        format!(
                            "Downloading game: {} / {} ",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::new(format!(
                        "Downloading game: {}",
                        format_file_size(download.current())
                    ))
                }
            }
            DownloadStatus::Comple => WListItem::new(format!(
                "Game downloaded: {}",
                format_file_size(download.current())
            )),
            DownloadStatus::Errored(err) => WListItem::new(format!("Game download error: {err}")),
        },
    }));

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
