use ratatui::{prelude::*, style::Style, symbols, widgets::LineGauge};
use tui_widget_list::{ListBuilder, ListState, ListView};

#[derive(Debug, Clone)]
pub struct ListItem {
    pub text: String,
    pub style: Style,
    pub item_type: ItemType,
}

#[derive(Debug, Clone)]
pub enum ItemType {
    Gauge(f64),
    Text,
}

impl ListItem {
    pub fn new<T: Into<String>>(text: T) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
            item_type: ItemType::Text,
        }
    }
    pub fn new_gauge<T: Into<String>>(text: T, ratio: f64) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
            item_type: ItemType::Gauge(ratio),
        }
    }
}

impl Widget for ListItem {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.item_type {
            ItemType::Text => Line::from(self.text).style(self.style).render(area, buf),
            ItemType::Gauge(ratio) => {
                let line_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(13), Constraint::Min(3)].as_ref())
                    .split(area);
                Line::from(self.text)
                    .style(self.style)
                    .render(line_layout[0], buf);
                LineGauge::default()
                    .filled_style(self.style.fg(Color::Black).bg(Color::White))
                    .line_set(symbols::line::THICK)
                    .ratio(ratio)
                    .render(line_layout[1], buf);
            }
        };
    }
}

pub struct App {
    state: ListState,
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let builder = ListBuilder::new(|context| {
            let mut item = ListItem::new(&format!("Item {:0}", context.index));

            // Alternating styles
            if context.index % 2 == 0 {
                item.style = Style::default().bg(Color::Rgb(28, 28, 32));
            } else {
                item.style = Style::default().bg(Color::Rgb(0, 0, 0));
            }

            // Style the selected element
            if context.is_selected {
                item.style = Style::default()
                    .bg(Color::Rgb(255, 153, 0))
                    .fg(Color::Rgb(28, 28, 32));
            };

            // Return the size of the widget along the main axis.
            let main_axis_size = 1;

            (item, main_axis_size)
        });

        let item_count = 2;
        let list = ListView::new(builder, item_count);
        let state = &mut self.state;

        list.render(area, buf, state);
    }
}
