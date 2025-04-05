use crate::SelectionList;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Widget, WidgetRef};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq)]
pub struct App {
    todo_lists: SelectionList<TodoList>,
    mode: Mode,
    key_mappings: HashMap<(Mode, KeyCode), Action>,
    quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            todo_lists: SelectionList::default(),
            mode: Mode::Normal,
            key_mappings: Self::default_key_mappings(),
            quit: false,
        }
    }
}

impl App {
    /// Consumes and runs application.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;
            self.update()?;
            if self.quit {
                return Ok(());
            }
        }
    }

    pub fn push_todo_list(&mut self, mut todo_list: TodoList) {
        if self.todo_lists.is_empty() {
            todo_list.is_selected = true;
        }
        self.todo_lists.push(todo_list);
    }

    /// Waits for user input, then updates state.
    fn update(&mut self) -> anyhow::Result<()> {
        let action = self.read_next_action()?;
        match action {
            Action::SetMode(mode) => self.mode = mode,
            Action::Quit => self.quit = true,
            Action::MoveLeft => self.select_previous_list(),
            Action::MoveRight => self.select_next_list(),
            Action::MoveUp => self.select_previous_todo(),
            Action::MoveDown => self.select_next_todo(),
            Action::MoveTop => self.select_top_todo(),
            Action::MoveBottom => self.select_bottom_todo(),
            Action::AddTodoAbove => self.add_todo(false),
            Action::AddTodoBelow => self.add_todo(true),
            Action::DeleteTodo => self.remove_todo(),
        }
        Ok(())
    }

    /// Draws user interface.
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let content = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height - 1,
        };
        let bottom_row = Rect {
            x: area.x,
            y: area.height - 1,
            width: area.width,
            height: 1,
        };
        let mode_text = match self.mode {
            Mode::Normal => "Normal",
            Mode::Insert => "Insert",
        };
        let constraints = vec![Constraint::Percentage(50); self.todo_lists.len()];
        let list_areas = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(constraints)
            .split(content);
        for (todo_list, area) in self.todo_lists.iter().zip(list_areas.iter().copied()) {
            frame.render_widget(todo_list, area);
        }
        frame.render_widget(mode_text, bottom_row);
    }

    /// Waits for the user to input motion, then returns the action corresponding to that key press.
    fn read_next_action(&self) -> anyhow::Result<Action> {
        loop {
            if let Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            }) = event::read()?
            {
                if let Some(action) = self.key_mappings.get(&(self.mode, code)) {
                    return Ok(*action);
                }
            }
        }
    }

    fn select_next_list(&mut self) {
        let Some(last_list) = self.todo_lists.selected_mut() else {
            return;
        };
        let last_todo_index = last_list.todos.selected_index().unwrap_or(0);
        last_list.is_selected = false;
        self.todo_lists.select_forwards_wrapping(1);
        let Some(list) = self.todo_lists.selected_mut() else {
            return;
        };
        list.todos.select(last_todo_index);
        list.is_selected = true;
    }

    fn select_previous_list(&mut self) {
        let Some(last_list) = self.todo_lists.selected_mut() else {
            return;
        };
        let last_todo_index = last_list.todos.selected_index().unwrap_or(0);
        last_list.is_selected = false;
        self.todo_lists.select_backwards_wrapping(1);
        let Some(list) = self.todo_lists.selected_mut() else {
            return;
        };
        list.todos.select(last_todo_index);
        list.is_selected = true;
    }

    fn select_previous_todo(&mut self) {
        let Some(selected_list) = self.todo_lists.selected_mut() else {
            return;
        };
        selected_list.todos.select_backwards(1);
    }

    fn select_next_todo(&mut self) {
        let Some(selected_list) = self.todo_lists.selected_mut() else {
            return;
        };
        selected_list.todos.select_forwards(1);
    }

    fn select_top_todo(&mut self) {
        let Some(selected_list) = self.todo_lists.selected_mut() else {
            return;
        };
        selected_list.todos.select_first();
    }

    fn select_bottom_todo(&mut self) {
        let Some(selected_list) = self.todo_lists.selected_mut() else {
            return;
        };
        selected_list.todos.select_last();
    }

    fn add_todo(&mut self, below: bool) {
        let Some(selected_list) = self.todo_lists.selected_mut() else {
            return;
        };
        let todos = &mut selected_list.todos;
        let selected_todo_index = todos.selected_index().unwrap_or(0);
        let index = if below {
            1 + selected_todo_index
        } else {
            selected_todo_index
        };
        todos.insert(index, "New Todo".to_owned());
        todos.select(index);
    }

    fn remove_todo(&mut self) {
        let Some(selected_list) = self.todo_lists.selected_mut() else {
            return;
        };
        let todos = &mut selected_list.todos;
        let Some(selected_todo_index) = todos.selected_index() else {
            return;
        };
        todos.remove(selected_todo_index);
    }

    fn default_key_mappings() -> HashMap<(Mode, KeyCode), Action> {
        let mut res = HashMap::new();
        res.insert((Mode::Normal, KeyCode::Char('o')), Action::AddTodoBelow);
        res.insert((Mode::Normal, KeyCode::Char('O')), Action::AddTodoAbove);
        res.insert((Mode::Normal, KeyCode::Char('D')), Action::DeleteTodo);
        res.insert((Mode::Normal, KeyCode::Char('h')), Action::MoveLeft);
        res.insert((Mode::Normal, KeyCode::Char('j')), Action::MoveDown);
        res.insert((Mode::Normal, KeyCode::Char('k')), Action::MoveUp);
        res.insert((Mode::Normal, KeyCode::Char('l')), Action::MoveRight);
        res.insert((Mode::Normal, KeyCode::Char('g')), Action::MoveTop);
        res.insert((Mode::Normal, KeyCode::Char('G')), Action::MoveBottom);
        res.insert((Mode::Normal, KeyCode::Char('q')), Action::Quit);
        res.insert((Mode::Normal, KeyCode::Left), Action::MoveLeft);
        res.insert((Mode::Normal, KeyCode::Down), Action::MoveDown);
        res.insert((Mode::Normal, KeyCode::Up), Action::MoveUp);
        res.insert((Mode::Normal, KeyCode::Right), Action::MoveRight);
        res.insert((Mode::Normal, KeyCode::Char('q')), Action::Quit);
        res.insert(
            (Mode::Normal, KeyCode::Char('i')),
            Action::SetMode(Mode::Insert),
        );
        res.insert((Mode::Insert, KeyCode::Esc), Action::SetMode(Mode::Normal));
        res
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    AddTodoAbove,
    AddTodoBelow,
    DeleteTodo,
    SetMode(Mode),
    Quit,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Mode {
    Normal,
    Insert,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TodoList {
    name: String,
    todos: SelectionList<String>,
    is_selected: bool,
}

impl TodoList {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            todos: SelectionList::default(),
            is_selected: false,
        }
    }

    pub fn push(&mut self, todo: impl Into<String>) {
        self.todos.push(todo.into());
    }

    pub fn insert(&mut self, index: usize, todo: impl Into<String>) {
        self.todos.insert(index, todo.into());
    }

    pub fn remove(&mut self, index: usize) {
        self.todos.remove(index);
    }

    pub fn todos(&self) -> &[String] {
        self.todos.elements()
    }

    pub fn todos_mut(&mut self) -> &mut [String] {
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
            Line::from(todo.as_str())
                .bg(bg_color)
                .fg(fg_color)
                .render(line_area, buf);
        }
    }
}

mod color {
    use crossterm::style::Color;

    pub const BG_UNSELECTED: Color = Color::Black;
    pub const FG_UNSELECTED: Color = Color::White;
    pub const BG_SELECTED: Color = Color::White;
    pub const FG_SELECTED: Color = Color::Black;
    pub const BORDER_UNSELECTED: Color = Color::White;
    pub const BORDER_SELECTED: Color = Color::Yellow;
}
