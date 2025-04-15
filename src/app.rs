use crate::{Todo, TodoList};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::{DefaultTerminal, Frame};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const APP_VERSION: & str = "0.1";

#[derive(Clone, Eq, PartialEq)]
pub struct App {
    config: Config,
    todo_lists: Vec<TodoList>,                      // All todo lists
    selection: Selection,                           // What is currently selected by the user
    mode: Mode,                                     // Mode of the app, influencing key presses.
    key_mappings: HashMap<(Mode, KeyCode), Action>, // Maps key presses to actions for a given mode
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
struct Config {
    /// Todo-list dabase path.
    dbpath: String,
}

/// State of the application, which is saved / loaded when the application starts and quits.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
struct State {
    version: String,
    todo_lists: Vec<TodoList>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: APP_VERSION.to_owned(),
            todo_lists: vec![
                TodoList {
                    name: "Todo".to_owned(),
                    todos: vec![],
                },
                TodoList {
                    name: "Backlog".to_owned(),
                    todos: vec![],
                },
            ],
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
struct Selection {
    todo_list: usize, // Todo list selected
    todo: usize,      // Todo in todo list selected
    char: usize,      // Index of character in todo selected, if any
}

impl App {
    /// Creates and initializes the application.
    pub fn init() -> anyhow::Result<Self> {
        let config = load_app_config()?;
        let dbpath = &config.dbpath;
        let state = match Path::new(dbpath).exists() {
            true => load_app_state(dbpath)?,
            false => State::default(),
        };
        Ok(Self {
            config,
            todo_lists: state.todo_lists,
            selection: Selection::default(),
            mode: Mode::Normal,
            key_mappings: default_key_mappings(),
        })
    }

    /// Consumes and runs application.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;
            let action = self.read_next_action()?;
            let quit = self.update(action)?;
            if quit {
                break;
            }
        }
        Ok(())
    }

    /// Waits for user input, then updates state.
    /// Returns true if application should quit.
    fn update(&mut self, action: Action) -> anyhow::Result<bool> {
        match action {
            Action::DeleteTodo => self.delete_todo(),
            Action::MoveTodoLeft => self.move_todo_left(),
            Action::MoveTodoRight => self.move_todo_right(),
            Action::MoveTodoUp => self.move_todo_up(),
            Action::MoveTodoDown => self.move_todo_down(),
            Action::Quit => {
                self.save()?;
                return Ok(true);
            }
            Action::SetMode(mode) => self.set_mode(mode),
            Action::MoveLeft => self.move_left(),
            Action::MoveRight => self.move_right(),
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::MoveTop => self.move_top(),
            Action::MoveBottom => self.move_bottom(),
            Action::AddTodoAbove => self.add_todo(false),
            Action::AddTodoBelow => self.add_todo(true),
            Action::Input(code) => self.input(code),
            Action::MoveCursorRight => self.move_cursor_right(),
            Action::MoveCursorLeft => self.move_cursor_left(),
            Action::MoveCursorStart => self.move_cursor_start(),
            Action::MoveCursorEnd => self.move_cursor_end(),
            Action::Nop => {}
        }
        Ok(false)
    }

    /// Draws user interface.
    fn render(&self, frame: &mut Frame) {
        // Computes areas to render in
        let area = frame.area();
        let content_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height - 1,
        };
        let bottom_area = Rect {
            x: area.x,
            y: area.height - 1,
            width: area.width,
            height: 1,
        };
        let constraints = vec![Constraint::Percentage(50); self.todo_lists.len()];
        let list_areas = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(constraints)
            .split(content_area);

        // Renders todo lists
        if !self.todo_lists.is_empty() {
            let sel_list_idx = self.selection.todo_list;
            let sel_list_idx = sel_list_idx.min(self.todo_lists.len() - 1);
            for (i, (todo_list, todo_list_area)) in self
                .todo_lists
                .iter()
                .zip(list_areas.iter().copied())
                .enumerate()
            {
                let is_list_selected = i == sel_list_idx;
                todo_list.render(
                    is_list_selected,
                    self.selection.todo,
                    self.selection.char,
                    self.mode,
                    todo_list_area,
                    frame,
                );
            }
        }

        // Renders bottom row
        let mode_text = match self.mode {
            Mode::Normal => "Normal",
            Mode::Insert => "Insert",
        };
        frame.render_widget(mode_text, bottom_area);
    }

    /// Waits for an event, input, then returns the corresponding action
    fn read_next_action(&self) -> anyhow::Result<Action> {
        loop {
            match event::read()? {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if let Some(action) = self.key_mappings.get(&(self.mode, code)) {
                        return Ok(*action);
                    } else if self.mode == Mode::Insert {
                        return Ok(Action::Input(code));
                    }
                }
                Event::Resize(_, _) => {
                    return Ok(Action::Nop);
                }
                _ => {}
            }
        }
    }

    /// Index of the currently selected todo list
    fn selected_todo_list(&self) -> Option<usize> {
        if self.todo_lists.is_empty() {
            return None;
        };
        Some(self.selection.todo_list)
    }

    /// Selects the desired todo list
    fn select_todo_list(&mut self, todo_list_idx: usize) {
        if todo_list_idx >= self.todo_lists.len() {
            return;
        }
        self.selection.todo_list = todo_list_idx;
    }

    /// Selects the desired todo
    fn select_todo(&mut self, todo_list_idx: usize, todo_idx: usize) {
        if todo_list_idx >= self.todo_lists.len() {
            return;
        }
        self.selection.todo_list = todo_list_idx;
        let todo_list = &mut self.todo_lists[todo_list_idx];
        if todo_idx >= todo_list.todos.len() {
            return;
        }
        self.selection.todo = todo_idx;
    }

    /// Indices of the currently selected todo
    fn selected_todo(&self) -> Option<(usize, usize)> {
        if self.todo_lists.is_empty() {
            return None;
        };
        let todo_list_idx = self.selection.todo_list;
        let todo_list = &self.todo_lists[todo_list_idx];
        if todo_list.todos.is_empty() {
            return None;
        };
        let todo_idx = self.selection.todo.min(todo_list.todos.len() - 1);
        Some((todo_list_idx, todo_idx))
    }

    fn set_mode(&mut self, mode: Mode) {
        if mode == Mode::Insert {
            let todo_list = &self.todo_lists[self.selection.todo_list];
            if todo_list.todos.is_empty() {
                return;
            }
            self.selection.char = 0;
        }
        self.mode = mode;
    }

    fn move_left(&mut self) {
        let Some(todo_list_idx) = self.selected_todo_list() else {
            return;
        };
        if todo_list_idx == 0 {
            return;
        };
        self.select_todo_list(todo_list_idx - 1);
    }

    fn move_right(&mut self) {
        let Some(todo_list_idx) = self.selected_todo_list() else {
            return;
        };
        self.select_todo_list(todo_list_idx + 1);
    }

    fn move_up(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_idx == 0 {
            return;
        };
        self.select_todo(todo_list_idx, todo_idx - 1);
    }

    fn move_down(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        self.select_todo(todo_list_idx, todo_idx + 1);
    }

    fn move_top(&mut self) {
        let Some(todo_list_idx) = self.selected_todo_list() else {
            return;
        };
        self.select_todo(todo_list_idx, 0);
    }

    fn move_bottom(&mut self) {
        let Some(todo_list_idx) = self.selected_todo_list() else {
            return;
        };
        let todo_list = &self.todo_lists[todo_list_idx];
        if todo_list.todos.is_empty() {
            return;
        };
        self.select_todo(todo_list_idx, todo_list.todos.len() - 1);
    }

    /// Inserts a [`Todo`] above or below the currently selected todo
    fn add_todo(&mut self, below: bool) {
        if self.todo_lists.is_empty() {
            return;
        };
        let todo_list = &mut self.todo_lists[self.selection.todo_list];
        let todos = &mut todo_list.todos;
        let todo_idx = match below {
            false => self.selection.todo.min(todos.len()),
            true => (self.selection.todo + 1).min(todos.len()),
        };
        todos.insert(todo_idx, Todo::new(""));
        self.selection.todo = todo_idx;
        self.set_mode(Mode::Insert);
    }

    /// Removes the currently selected [`Todo`]
    fn delete_todo(&mut self) {
        if self.todo_lists.is_empty() {
            return;
        };
        let todo_list = &mut self.todo_lists[self.selection.todo_list];
        let todos = &mut todo_list.todos;
        if todos.is_empty() {
            return;
        };
        let todo_idx = self.selection.todo.min(todos.len() - 1);
        todos.remove(todo_idx);
    }

    fn move_todo_left(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_list_idx == 0 {
            return;
        };
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = todo_list.todos.remove(todo_idx);
        let next_todo_list = &mut self.todo_lists[todo_list_idx - 1];
        let next_todo_idx = self.selection.todo.min(next_todo_list.todos.len());
        next_todo_list.todos.insert(next_todo_idx, todo);
        self.selection.todo_list -= 1;
    }

    fn move_todo_right(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_list_idx == self.todo_lists.len() - 1 {
            return;
        };
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = todo_list.todos.remove(todo_idx);
        let next_todo_list = &mut self.todo_lists[todo_list_idx + 1];
        let next_todo_idx = self.selection.todo.min(next_todo_list.todos.len());
        next_todo_list.todos.insert(next_todo_idx, todo);
        self.selection.todo_list += 1;
    }

    fn move_todo_up(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_idx == 0 {
            return;
        };
        let todo_list = &mut self.todo_lists[todo_list_idx];
        todo_list.todos.swap(todo_idx, todo_idx - 1);
        self.select_todo(todo_list_idx, todo_idx - 1);
    }

    fn move_todo_down(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        let todo_list = &mut self.todo_lists[todo_list_idx];
        if todo_idx == todo_list.todos.len() - 1 {
            return;
        };
        todo_list.todos.swap(todo_idx, todo_idx + 1);
        self.select_todo(todo_list_idx, todo_idx + 1);
    }

    /// Inputs a character to the name of the currently selected [`Todo`].
    fn input(&mut self, code: KeyCode) {
        if self.todo_lists.is_empty() {
            return;
        };
        let todo_list = &mut self.todo_lists[self.selection.todo_list];
        let todos = &mut todo_list.todos;
        if todos.is_empty() {
            return;
        };
        let todo_idx = self.selection.todo.min(todos.len() - 1);
        let todo = &mut todos[todo_idx];
        let char_index = self.selection.char;
        match code {
            KeyCode::Char(c) => {
                todo.name.insert(char_index, c);
                self.selection.char += 1;
            }
            KeyCode::Backspace => {
                if self.selection.char > 0 {
                    todo.name.remove(char_index - 1);
                    self.selection.char -= 1;
                }
            }
            KeyCode::Delete => {
                if self.selection.char < todo.name.len() {
                    todo.name.remove(char_index);
                }
            }
            _ => {}
        }
    }

    fn move_cursor_right(&mut self) {
        let Some(todo_list) = self.todo_lists.get(self.selection.todo_list) else {
            return;
        };
        let todo = &todo_list.todos[self.selection.todo];
        if self.selection.char >= todo.name.len() {
            return;
        };
        self.selection.char += 1;
    }

    fn move_cursor_left(&mut self) {
        if self.selection.char == 0 {
            return;
        };
        self.selection.char -= 1;
    }

    fn move_cursor_start(&mut self) {
        self.selection.char = 0;
    }

    fn move_cursor_end(&mut self) {
        let Some(todo_list) = self.todo_lists.get(self.selection.todo_list) else {
            return;
        };
        let todo = &todo_list.todos[self.selection.todo];
        self.selection.char = todo.name.len();
    }

    fn save(&self) -> anyhow::Result<()> {
        let dbpath = Path::new(&self.config.dbpath);
        if let Some(parent) = dbpath.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let state = State {
            todo_lists: self.todo_lists.clone(),
            ..Default::default()
        };
        let state_str = serde_yaml::to_string(&state)?;
        std::fs::write(dbpath, state_str)?;
        Ok(())
    }
}

fn default_key_mappings() -> HashMap<(Mode, KeyCode), Action> {
    let mut res = HashMap::new();
    res.insert((Mode::Normal, KeyCode::Char('o')), Action::AddTodoBelow);
    res.insert((Mode::Normal, KeyCode::Char('O')), Action::AddTodoAbove);
    res.insert((Mode::Normal, KeyCode::Char('x')), Action::DeleteTodo);
    res.insert((Mode::Normal, KeyCode::Char('H')), Action::MoveTodoLeft);
    res.insert((Mode::Normal, KeyCode::Char('J')), Action::MoveTodoDown);
    res.insert((Mode::Normal, KeyCode::Char('K')), Action::MoveTodoUp);
    res.insert((Mode::Normal, KeyCode::Char('L')), Action::MoveTodoRight);
    res.insert((Mode::Normal, KeyCode::Char('h')), Action::MoveLeft);
    res.insert((Mode::Normal, KeyCode::Char('j')), Action::MoveDown);
    res.insert((Mode::Normal, KeyCode::Char('k')), Action::MoveUp);
    res.insert((Mode::Normal, KeyCode::Char('l')), Action::MoveRight);
    res.insert((Mode::Normal, KeyCode::Char('g')), Action::MoveTop);
    res.insert((Mode::Normal, KeyCode::Char('G')), Action::MoveBottom);
    res.insert((Mode::Normal, KeyCode::Home), Action::MoveTop);
    res.insert((Mode::Normal, KeyCode::End), Action::MoveBottom);
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
    res.insert((Mode::Insert, KeyCode::Right), Action::MoveCursorRight);
    res.insert((Mode::Insert, KeyCode::Left), Action::MoveCursorLeft);
    res.insert((Mode::Insert, KeyCode::Home), Action::MoveCursorStart);
    res.insert((Mode::Insert, KeyCode::End), Action::MoveCursorEnd);
    res
}

fn load_app_config() -> anyhow::Result<Config> {
    let home_dir = std::env::var("HOME")?;
    let config_dir = format!("{home_dir}/.config/tdi");
    std::fs::create_dir_all(&config_dir)?;
    let config_path = format!("{config_dir}/config.yml");
    if !std::fs::exists(&config_path)? {
        Ok(Config {
            dbpath: format!("{home_dir}/.local/share/tdi/db.yml"),
        })
    } else {
        let config_str: String = std::fs::read_to_string(config_path)?;
        let config: Config = serde_yaml::from_str(&config_str)?;
        Ok(config)
    }
}

fn load_app_state(dbpath: &str) -> anyhow::Result<State> {
    let state_string = std::fs::read_to_string(dbpath)?;
    let state = serde_yaml::from_str(&state_string)?;
    Ok(state)
}

/// Value that causes an [`App`] to perform an action.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Action {
    DeleteTodo,
    MoveTodoLeft,
    MoveTodoRight,
    MoveTodoUp,
    MoveTodoDown,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    AddTodoAbove,
    AddTodoBelow,
    Input(KeyCode),
    SetMode(Mode),
    MoveCursorRight,
    MoveCursorLeft,
    MoveCursorStart,
    MoveCursorEnd,
    Nop, // No operation. Useful if app needs to rerender.
    Quit,
}

/// Current mode of an [`App`] which determines the action keys map to.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum Mode {
    /// Initial mode, allowing user to navigate and move todo lists.
    Normal,
    /// Mode when inserting a value in the cell of a todo.
    Insert,
}
