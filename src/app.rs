use crate::SelectionList;
use crate::{Todo, TodoList};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;

#[derive(Clone, Eq, PartialEq)]
pub struct App {
    todo_lists: SelectionList<TodoList>, // All todo lists, laid out horizontally
    mode: Mode,                          // Mode of the app, influencing key presses.
    quit: bool,                          // App will quit when this value is set to true.
    key_mappings: HashMap<(Mode, KeyCode), Action>, // Maps key presses to actions for a given mode
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
            Action::AddTodoAbove => self.add_todo(Todo::new("Todo Name"), false),
            Action::AddTodoBelow => self.add_todo(Todo::new("Todo Name"), true),
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

    fn add_todo(&mut self, todo: Todo, below: bool) {
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
        todos.insert(index, todo);
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

/// Value that causes an [`App`] to perform an action.
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

/// Current mode of an [`App`] which determines the action keys map to.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum Mode {
    /// Initial mode, allowing user to navigate todo lists.
    Normal,
    /// Mode when inserting a value in the cell of a todo.
    Insert,
}
