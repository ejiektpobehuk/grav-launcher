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
    widgets::{Block, List, ListItem, ListState},
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
        }
    }
    
    pub fn next_log(&mut self) {
        self.focused_log = match self.focused_log {
            FocusedLog::LauncherLog => FocusedLog::GameStdout,
            FocusedLog::GameStdout => FocusedLog::GameStderr,
            FocusedLog::GameStderr => FocusedLog::LauncherLog,
        };
    }
    
    pub fn prev_log(&mut self) {
        self.focused_log = match self.focused_log {
            FocusedLog::LauncherLog => FocusedLog::GameStderr,
            FocusedLog::GameStdout => FocusedLog::LauncherLog,
            FocusedLog::GameStderr => FocusedLog::GameStdout,
        };
    }
}

pub fn draw(frame: &mut Frame, app_state: &mut AppState) {
    let area = frame.area();

    let outer_layout = Layout::default()
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(area);

    let title = Line::from(" GRAV launcher ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);
    frame.render_widget(block, area);

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
                            (download.current() as f64) / (total.clone() as f64),
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
