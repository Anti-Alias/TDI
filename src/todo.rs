use crate::color;
use crate::SelectionList;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Widget, WidgetRef};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TodoList {
    pub name: String,
    pub todos: SelectionList<Todo>,
    pub is_selected: bool,
}

impl TodoList {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            todos: SelectionList::default(),
            is_selected: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn push(&mut self, todo: Todo) {
        self.todos.push(todo);
    }

    pub fn insert(&mut self, index: usize, todo: Todo) {
        self.todos.insert(index, todo);
    }

    pub fn remove(&mut self, index: usize) {
        self.todos.remove(index);
    }

    pub fn todos(&self) -> &[Todo] {
        self.todos.elements()
    }

    pub fn todos_mut(&mut self) -> &mut [Todo] {
        self.todos.elements_mut()
    }

    pub fn select_previous(&mut self) {
        self.todos.select_backwards(1);
    }

    pub fn select_next(&mut self) {
        self.todos.select_forwards(1);
    }
}

impl WidgetRef for TodoList {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let color = if self.is_selected {
            color::BORDER_SELECTED
        } else {
            color::BORDER_UNSELECTED
        };
        Block::default()
            .title(self.name.as_ref())
            .borders(Borders::all())
            .title_alignment(Alignment::Center)
            .fg(color)
            .render_ref(area, buf);
        let mut line_area = area;
        line_area.x += 2;
        line_area.width -= 4;
        line_area.height = 1;
        for (i, todo) in self.todos.iter().enumerate() {
            let is_todo_selected = self.is_selected && i == self.todos.selected_index().unwrap();
            let (bg_color, fg_color) = match is_todo_selected {
                false => (color::BG_UNSELECTED, color::FG_UNSELECTED),
                true => (color::BG_SELECTED, color::FG_SELECTED),
            };
            line_area.y += 1;
            Line::from(todo.name.as_str())
                .bg(bg_color)
                .fg(fg_color)
                .render(line_area, buf);
        }
    }
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct Todo {
    pub name: String,
}

impl Todo {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
