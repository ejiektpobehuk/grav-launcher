mod log;
use crate::ui::log::Log;

use ratatui::{
    Frame,
    prelude::*,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, List, ListItem, ListState},
};

pub struct AppState {
    pub log: Log,
    pub game_stdout: Vec<String>,
    pub game_stderr: Vec<String>,
    pub list_state: ListState,
    pub stdout_state: ListState,
    pub stderr_state: ListState,
}

impl AppState {
    pub fn init() -> Self {
        Self {
            log: Log::new(),
            game_stdout: Vec::new(),
            game_stderr: Vec::new(),
            list_state: ListState::default(),
            stdout_state: ListState::default(),
            stderr_state: ListState::default(),
        }
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

    let items: Vec<ListItem> = app_state
        .log
        .entries()
        .iter()
        .map(|i| {
            let content = Line::from(Span::raw(i).to_string());
            ListItem::new(content)
        })
        .collect();

    let title = Line::from(" Launcher log ".bold());
    let block = Block::bordered()
        .title(title.centered())
        .border_set(border::THICK);

    let list = List::new(items).block(block);

    frame.render_stateful_widget(list, inner_layout[0], &mut app_state.list_state);

    let stdouts: Vec<ListItem> = app_state
        .game_stdout
        .iter()
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

    frame.render_stateful_widget(stdout, game_output_layout[0], &mut app_state.stdout_state);

    let stderrs: Vec<ListItem> = app_state
        .game_stderr
        .iter()
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

    frame.render_stateful_widget(stderr, game_output_layout[1], &mut app_state.stderr_state);
}
