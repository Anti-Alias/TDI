use crate::{Mode, color};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TodoList {
    pub name: String,
    pub todos: Vec<Todo>,
}

impl TodoList {

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            todos: Vec::default(),
        }
    }

    pub fn render(&self,
        is_selected: bool,
        todo_selected: usize,
        char_selected: usize,
        mode: Mode,
        area: Rect,
        frame: &mut Frame,
    ) { 
        // Todo container
        let color = if is_selected { color::BORDER_SELECTED } else { color::BORDER_UNSELECTED };
        let block = Block::default()
            .title(self.name.as_ref())
            .borders(Borders::all())
            .title_alignment(Alignment::Center)
            .fg(color);
        frame.render_widget(block, area);

        // Todos
        let mut line_area = area;
        line_area.x += 2;
        if !self.todos.is_empty() {
            line_area.width -= 4;
            line_area.height = 1;
            let todo_selected = todo_selected.min(self.todos.len()-1);
            for (i, todo) in self.todos.iter().enumerate() {
                let is_todo_selected = mode == Mode::Normal && is_selected && i == todo_selected;
                let (bg_color, fg_color) = match is_todo_selected {
                    false => (color::BG_UNSELECTED, color::FG_UNSELECTED),
                    true => (color::BG_SELECTED, color::FG_SELECTED),
                };
                line_area.y += 1;
                let todo_name = if todo.name.is_empty() { " " } else { &todo.name };
                let line = Line::from(todo_name).bg(bg_color).fg(fg_color);
                frame.render_widget(line, line_area);
            }
        }

        // Sets cursor position
        if mode == Mode::Insert && is_selected {
            let cursor_x = 2 + area.x + char_selected as u16;
            let cursor_y = 1 + area.y + todo_selected as u16;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// A single todo in a [`TodoList`]
#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct Todo {
    pub name: String,
}

impl Todo {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
