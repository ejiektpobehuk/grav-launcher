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
}

pub fn draw(frame: &mut Frame, app_state: &mut AppState) {
    let area = frame.area();

    // Create help text for controls
    let help_text = if app_state.show_exit_popup {
        // Hide normal controls when popup is shown
        vec![]
    } else if app_state.fullscreen_mode {
        if app_state.terminal_focused {
            vec![
                Span::raw(" Press "),
                Span::styled("B", Style::default().fg(Color::Red).bold()),
                Span::raw("/"),
                Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
                Span::raw("/"),
                Span::styled("h", Style::default().fg(Color::Blue).bold()),
                Span::raw(" to return to normal view "),
            ]
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
        vec![
            Span::styled(" A", Style::default().fg(Color::Green).bold()),
            Span::raw("/"),
            Span::styled("Enter", Style::default().fg(Color::Blue).bold()),
            Span::raw(" Fullscreen | "),
            Span::styled("B", Style::default().fg(Color::Red).bold()),
            Span::raw("/"),
            Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
            Span::raw(" Exit | "),
            Span::styled("D-Pad", Style::default().fg(Color::Yellow).bold()),
            Span::raw("/"),
            Span::styled("Arrows", Style::default().fg(Color::Blue).bold()),
            Span::raw(" Navigate "),
        ]
    };

    let help_line = Line::from(help_text);

    // Main layout that uses the full area
    let outer_layout = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area);

    let title = Line::from(" GRAV launcher ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .title_bottom(help_line.right_aligned())
        .border_set(border::THICK);
    frame.render_widget(block, area);

    if app_state.fullscreen_mode {
        // Fullscreen mode - show only the focused log
        let content_area = Layout::default()
            .margin(2)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(outer_layout[0])[0];

        match app_state.focused_log {
            FocusedLog::LauncherLog => {
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
                            DownloadStatus::Comple => WListItem::new(Line::from(Span::raw(
                                format!("Downloaded: {} bytes", download.current()),
                            ))),
                            DownloadStatus::Errored(err) => {
                                WListItem::new(format!("Download error: {err}"))
                            }
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
                frame.render_stateful_widget(list, content_area, &mut app_state.list_state);
            }
            FocusedLog::GameStdout => {
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
                frame.render_stateful_widget(stdout, content_area, &mut app_state.stdout_state);
            }
            FocusedLog::GameStderr => {
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
                frame.render_stateful_widget(stderr, content_area, &mut app_state.stderr_state);
            }
        }
    } else {
        // Normal mode - show all logs
        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .margin(2)
            .constraints(vec![Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(outer_layout[0]);

        let game_output_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner_layout[1]);

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
                    DownloadStatus::Errored(err) => {
                        WListItem::new(format!("Download error: {err}"))
                    }
                    DownloadStatus::NotStarted => {
                        WListItem::new("Download oopsie: something strange happened")
                    }
                },
            })
            .collect();

        let builder = ListBuilder::new(|context| {
            let item = items[context.index].clone();

            // Return the size of the widget along the main axis.
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

        frame.render_stateful_widget(list, inner_layout[0], &mut app_state.list_state);

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

        frame.render_stateful_widget(stdout, game_output_layout[0], &mut app_state.stdout_state);

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

        frame.render_stateful_widget(stderr, game_output_layout[1], &mut app_state.stderr_state);
    }

    // Render exit confirmation popup if needed
    if app_state.show_exit_popup {
        let popup_area = centered_rect(46, 12, area);

        // Create a popup with text and border
        let popup_block = Block::default()
            .title(" Exit Confirmation ".bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .border_type(BorderType::Rounded);

        // Controls text to display in the popup
        let controls_text = vec![
            Span::styled(" A", Style::default().fg(Color::Green).bold()),
            Span::raw("/"),
            Span::styled("Enter", Style::default().fg(Color::Blue).bold()),
            Span::raw("/"),
            Span::styled("y", Style::default().fg(Color::Blue).bold()),
            Span::raw(" - Yes    "),
            Span::styled("B", Style::default().fg(Color::Red).bold()),
            Span::raw("/"),
            Span::styled("Esc", Style::default().fg(Color::Blue).bold()),
            Span::raw("/"),
            Span::styled("n", Style::default().fg(Color::Blue).bold()),
            Span::raw(" - No "),
        ];

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
