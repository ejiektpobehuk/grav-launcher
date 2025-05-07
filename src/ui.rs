pub mod log;
use crate::ui::log::{Entry, Log};
mod list;
use crate::ui::list::ListItem as WListItem;

use log::DownloadStatus;
use ratatui::{
    Frame,
    prelude::*,
    style::{Color, Style, Stylize},
    symbols::{border, scrollbar},
    text::Line,
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
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
    Fullscreen(usize),
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
    pub stdout_scroll: usize,
    pub stderr_scroll: usize,
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
            stdout_scroll: 0,
            stderr_scroll: 0,
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

    pub fn enter_fullscreen(&mut self, visible_height: usize) {
        self.display_mode = DisplayMode::Fullscreen(visible_height);
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

    pub fn scroll_up(&mut self) {
        match self.focused_log {
            FocusedLog::GameStdout => {
                if self.stdout_scroll > 0 {
                    self.stdout_scroll = self.stdout_scroll.saturating_sub(1);
                }
            }
            FocusedLog::GameStderr => {
                if self.stderr_scroll > 0 {
                    self.stderr_scroll = self.stderr_scroll.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    pub fn scroll_down(&mut self) {
        match self.focused_log {
            FocusedLog::GameStdout => {
                let max_scroll = self.game_stdout.len().saturating_sub(1);
                if self.stdout_scroll < max_scroll {
                    self.stdout_scroll = self.stdout_scroll.saturating_add(1);
                }
            }
            FocusedLog::GameStderr => {
                let max_scroll = self.game_stderr.len().saturating_sub(1);
                if self.stderr_scroll < max_scroll {
                    self.stderr_scroll = self.stderr_scroll.saturating_add(1);
                }
            }
            _ => {}
        }
    }
}

pub fn draw(frame: &mut Frame, app_state: &mut AppState) {
    let area = frame.area();

    // Calculate visible height for fullscreen mode
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    // Update fullscreen mode with current visible height
    if let DisplayMode::Fullscreen(_) = app_state.display_mode {
        app_state.enter_fullscreen(visible_height);
    }

    // Render the main UI frame with title and help text
    render_main_frame(frame, area, app_state);

    if app_state.display_mode != DisplayMode::Normal {
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
    } else if let DisplayMode::Fullscreen(visible_height) = app_state.display_mode {
        let mut controls = Vec::new();

        // Add scrolling instructions if content is scrollable
        let is_scrollable = match app_state.focused_log {
            FocusedLog::LauncherLog => app_state.log.entries().len() > visible_height,
            FocusedLog::GameStdout => app_state.game_stdout.len() > visible_height,
            FocusedLog::GameStderr => app_state.game_stderr.len() > visible_height,
        };

        if is_scrollable {
            controls.push(Span::raw(" "));
            match app_state.input_method {
                InputMethod::Controller => {
                    controls.push(Span::styled(
                        "D-Pad Up/Down",
                        Style::default().fg(Color::Yellow).bold(),
                    ));
                    controls.push(Span::raw(" Scroll "));
                }
                InputMethod::Keyboard => {
                    controls.push(Span::styled("↑/↓", Style::default().fg(Color::Blue).bold()));
                    controls.push(Span::raw(" Scroll "));
                }
            }
            controls.push(Span::raw(" |"));
        }

        // Add back control
        match app_state.input_method {
            InputMethod::Controller => {
                controls.push(Span::styled(" B", Style::default().fg(Color::Red).bold()));
                controls.push(Span::raw(" Back "));
            }
            InputMethod::Keyboard => {
                controls.push(Span::styled(
                    " Esc",
                    Style::default().fg(Color::Blue).bold(),
                ));
                controls.push(Span::raw(" Back "));
            }
        }

        controls
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
        Entry::Text(title_opt, text) => match title_opt {
            Some(title) => WListItem::with_title(title, text),
            None => WListItem::new(text),
        },
        Entry::Downloand(download) => {
            WListItem::with_title("Download", format_file_size(download.current()))
        }
        Entry::LauncherUpdate(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        "Launcher update",
                        format!(
                            "{} / {}",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::with_title("Launcher update", format_file_size(download.current()))
                }
            }
            DownloadStatus::Comple => WListItem::with_title(
                "Launcher update",
                format!(
                    "{} Downloaded. Restart needed.",
                    format_file_size(download.current())
                ),
            ),
            DownloadStatus::Errored(err) => WListItem::with_title("Launcher update error", err),
        },
        Entry::GameDownload(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        "Downloading game",
                        format!(
                            "{} / {}",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::with_title("Downloading game", format_file_size(download.current()))
                }
            }
            DownloadStatus::Comple => {
                WListItem::with_title("Game downloaded", format_file_size(download.current()))
            }
            DownloadStatus::Errored(err) => WListItem::with_title("Game download error", err),
        },
    }));

    let builder = ListBuilder::new(|context| {
        let item = items[context.index].clone();
        let main_axis_size = 1;
        (item, main_axis_size)
    });

    let title = Line::from(" Launcher log ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let list = ListView::new(builder, items.len()).block(block);
    frame.render_stateful_widget(list, area, &mut app_state.list_state);
}

fn render_fullscreen_game_stdout(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    let total_items = app_state.game_stdout.len();

    // Calculate max scroll position - when last line is visible
    let max_scroll = if total_items <= visible_height {
        0
    } else {
        total_items.saturating_sub(visible_height)
    };

    // Ensure scroll position doesn't exceed max
    app_state.stdout_scroll = app_state.stdout_scroll.min(max_scroll);

    let start_idx = app_state.stdout_scroll;
    let end_idx = (start_idx + visible_height).min(total_items);

    let stdouts: Vec<ListItem> = app_state
        .game_stdout
        .iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .map(|i| {
            let content = Line::from(Span::raw(i.to_string()));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Game text output ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let stdout = List::new(stdouts).block(block);
    frame.render_stateful_widget(stdout, area, &mut app_state.stdout_state);

    // Add scrollbar integrated into the border
    if total_items > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None);

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll + 1) // +1 because we want to include the last position
            .viewport_content_length(visible_height)
            .position(start_idx);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn render_fullscreen_game_stderr(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders
    let total_items = app_state.game_stderr.len();

    // Calculate max scroll position - when last line is visible
    let max_scroll = if total_items <= visible_height {
        0
    } else {
        total_items.saturating_sub(visible_height)
    };

    // Ensure scroll position doesn't exceed max
    app_state.stderr_scroll = app_state.stderr_scroll.min(max_scroll);

    let start_idx = app_state.stderr_scroll;
    let end_idx = (start_idx + visible_height).min(total_items);

    let stderrs: Vec<ListItem> = app_state
        .game_stderr
        .iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .map(|i| {
            let content = Line::from(Span::raw(i.to_string()));
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Game errors ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let stderr = List::new(stderrs).block(block);
    frame.render_stateful_widget(stderr, area, &mut app_state.stderr_state);

    // Add scrollbar integrated into the border
    if total_items > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None);

        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll + 1) // +1 because we want to include the last position
            .viewport_content_length(visible_height)
            .position(start_idx);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
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
        Entry::Text(title_opt, text) => match title_opt {
            Some(title) => WListItem::with_title(title, text),
            None => WListItem::new(text),
        },
        Entry::Downloand(download) => {
            WListItem::with_title("Download", format_file_size(download.current()))
        }
        Entry::LauncherUpdate(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        "Launcher update",
                        format!(
                            "{} / {}",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::with_title("Launcher update", format_file_size(download.current()))
                }
            }
            DownloadStatus::Comple => WListItem::with_title(
                "Launcher update",
                format!(
                    "{} Downloaded. Restart needed.",
                    format_file_size(download.current())
                ),
            ),
            DownloadStatus::Errored(err) => WListItem::with_title("Launcher update error", err),
        },
        Entry::GameDownload(download) => match download.status() {
            DownloadStatus::InProgress => {
                if let Some(total) = download.total() {
                    WListItem::new_gauge(
                        "Downloading game",
                        format!(
                            "{} / {}",
                            format_file_size(download.current()),
                            format_file_size(*total)
                        ),
                        (download.current() as f64) / (*total as f64),
                    )
                } else {
                    WListItem::with_title("Downloading game", format_file_size(download.current()))
                }
            }
            DownloadStatus::Comple => {
                WListItem::with_title("Game downloaded", format_file_size(download.current()))
            }
            DownloadStatus::Errored(err) => WListItem::with_title("Game download error", err),
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
    let visible_height = area.height as usize;
    let total_items = app_state.game_stdout.len();

    // Calculate visible range to show the bottom part
    let start_idx = if total_items <= visible_height {
        0
    } else {
        total_items.saturating_sub(visible_height)
    };
    let end_idx = total_items;

    let stdouts: Vec<ListItem> = app_state
        .game_stdout
        .iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
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

    // Add scrollbar if there's more content than can be displayed
    if total_items > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None);

        let max_scroll = total_items.saturating_sub(visible_height);
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll + 1)
            .viewport_content_length(visible_height)
            .position(start_idx);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn render_game_stderr(frame: &mut Frame, area: Rect, app_state: &mut AppState) {
    let visible_height = area.height as usize;
    let total_items = app_state.game_stderr.len();

    // Calculate visible range to show the bottom part
    let start_idx = if total_items <= visible_height {
        0
    } else {
        total_items.saturating_sub(visible_height)
    };
    let end_idx = total_items;

    let stderrs: Vec<ListItem> = app_state
        .game_stderr
        .iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
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

    // Add scrollbar if there's more content than can be displayed
    if total_items > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .symbols(scrollbar::VERTICAL)
            .begin_symbol(None)
            .track_symbol(None)
            .end_symbol(None);

        let max_scroll = total_items.saturating_sub(visible_height);
        let mut scrollbar_state = ScrollbarState::default()
            .content_length(max_scroll + 1)
            .viewport_content_length(visible_height)
            .position(start_idx);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn render_exit_popup(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let popup_area = centered_rect(46, 12, area);

    // Controls text to display in the popup
    let controls_text = match app_state.input_method {
        InputMethod::Controller => Line::from(vec![
            Span::styled(" A", Style::default().fg(Color::Green).bold()),
            Span::raw(" - Yes    "),
            Span::styled("B", Style::default().fg(Color::Red).bold()),
            Span::raw(" - No "),
        ]),
        InputMethod::Keyboard => Line::from(vec![
            Span::styled(" Enter", Style::default().fg(Color::Blue).bold()),
            Span::raw(" - ("),
            Span::styled("Y", Style::default().fg(Color::Blue).bold()),
            Span::raw(")es | "),
            Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
            Span::raw(" - ("),
            Span::styled("N", Style::default().fg(Color::Blue).bold()),
            Span::raw(")o "),
        ]),
    };

    // Create a popup with no title and controls in the border
    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .border_type(BorderType::Rounded)
        .title_bottom(controls_text.right_aligned());

    let popup_text = Paragraph::new(vec![
        Line::from(""),
        Line::from("Are you sure you want to exit?"),
        Line::from(""),
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
