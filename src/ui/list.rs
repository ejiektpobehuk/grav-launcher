use ratatui::{prelude::*, style::Style, symbols, widgets::LineGauge};
use tui_widget_list::{ListBuilder, ListState, ListView};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub struct ListItem {
    pub title: Option<String>,
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
            title: None,
            text: text.into(),
            style: Style::default(),
            item_type: ItemType::Text,
        }
    }

    pub fn with_title<T: Into<String>, U: Into<String>>(title: T, text: U) -> Self {
        Self {
            title: Some(title.into()),
            text: text.into(),
            style: Style::default(),
            item_type: ItemType::Text,
        }
    }

    pub fn new_gauge<T: Into<String>, U: Into<String>>(title: T, text: U, ratio: f64) -> Self {
        Self {
            title: Some(title.into()),
            text: text.into(),
            style: Style::default(),
            item_type: ItemType::Gauge(ratio),
        }
    }
}

impl Widget for ListItem {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.item_type {
            ItemType::Text => {
                let mut line = Line::default();
                match self.title {
                    Some(title) => {
                        let title_span = Span::styled(title, self.style.bold());
                        let text_span = if self.text.is_empty() {
                            Span::styled("", self.style)
                        } else {
                            Span::styled(format!(": {}", self.text), self.style)
                        };
                        line.spans = vec![title_span, text_span];
                    }
                    None => {
                        line.spans = vec![Span::styled(self.text, self.style)];
                    }
                }
                line.render(area, buf);
            }
            ItemType::Gauge(ratio) => {
                let title = match self.title {
                    Some(t) => format!("{t}: "),
                    None => String::new(),
                };
                let title_width = title.width() as u16;
                let text_width = self.text.width() as u16;
                let line_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(title_width),
                            Constraint::Min(3),
                            Constraint::Length(text_width),
                        ]
                        .as_ref(),
                    )
                    .split(area);

                if !title.is_empty() {
                    Line::from(title)
                        .style(self.style.bold())
                        .render(line_layout[0], buf);
                }
                LineGauge::default()
                    .filled_style(self.style.fg(Color::Black).bg(Color::White))
                    .line_set(symbols::line::THICK)
                    .ratio(ratio)
                    .render(line_layout[1], buf);
                if !self.text.is_empty() {
                    Line::from(self.text)
                        .style(self.style)
                        .render(line_layout[2], buf);
                }
            }
        }
    }
}

pub struct App {
    state: ListState,
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let builder = ListBuilder::new(|context| {
            let mut item = if context.index % 2 == 0 {
                ListItem::with_title(
                    format!("Title {}", context.index),
                    format!("Text {}", context.index),
                )
            } else {
                ListItem::new(format!("Text {}", context.index))
            };

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
            }

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
