use crate::{Todo, TodoList};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::{DefaultTerminal, Frame};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::Path;

const APP_VERSION: & str = "0.1";
const BACKLOG_LIST_IDX: usize = 1;
const MOVE_HALF_AMOUNT: usize = 5;


#[derive(Clone, Eq, PartialEq)]
pub struct App {
    config: Config,
    todo_lists: Vec<TodoList>,                      // All todo lists.
    selection: Selection,                           // What is currently selected by the user.
    mode: Mode,                                     // Mode of the app, influencing key presses.
    key_mappings: HashMap<KeyPress, Action>,        // Maps key presses to actions while in a given mode.
    snapshots: VecDeque<State>,                     // Snapshots of the app's state, used for undo/redo functionality.
    needs_saving: bool,                             // Set to true if a change occurred, requiring saving.
    current_snapshot: usize, 
    max_snapshots: usize, 
    quit: bool,
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
            snapshots: VecDeque::new(),
            needs_saving: false,
            current_snapshot: 0,
            max_snapshots: 100,
            quit: false,
        })
    }

    /// Consumes and runs application.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> anyhow::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;
            let action = self.read_next_action()?;
            self.update(action)?;
            if self.quit {
                break;
            }
        }
        Ok(())
    }

    /// Waits for an event, input, then returns the corresponding action
    fn read_next_action(&self) -> anyhow::Result<Action> {
        loop {
            match event::read()? {
                Event::Key(KeyEvent { code, kind: KeyEventKind::Press, modifiers, .. }) => {
                    let key_press = KeyPress { mode: self.mode, code, modifiers };
                    if let Some(action) = self.key_mappings.get(&key_press) {
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

    /// Waits for user input, then updates state.
    /// Returns true if application should quit.
    fn update(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::Quit => self.quit()?,
            Action::DeleteTodo => self.delete_todo(),
            Action::MoveTodoLeft => self.move_todo_left(),
            Action::MoveTodoRight => self.move_todo_right(),
            Action::MoveTodoUp => self.move_todo_up(),
            Action::MoveTodoDown => self.move_todo_down(),
            Action::SetMode(mode) => self.set_mode(mode),
            Action::MoveLeft => self.move_left(),
            Action::MoveRight => self.move_right(),
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::MoveUpHalf => self.move_up_half(),
            Action::MoveDownHalf => self.move_down_half(),
            Action::MoveTop => self.move_top(),
            Action::MoveBottom => self.move_bottom(),
            Action::AddTodoAbove => self.add_todo(false),
            Action::AddTodoBelow => self.add_todo(true),
            Action::ToggleMark => self.toggle_mark(),
            Action::Input(code) => self.input(code),
            Action::MoveCursorRight => self.move_cursor_right(),
            Action::MoveCursorLeft => self.move_cursor_left(),
            Action::MoveCursorStart => self.move_cursor_start(),
            Action::MoveCursorEnd => self.move_cursor_end(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Nop => {}
        }
        Ok(())
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
            let todo_list_idx = self.selection.todo_list;
            let todo_list_idx = todo_list_idx.min(self.todo_lists.len() - 1);
            for (i, (todo_list, todo_list_area)) in self
                .todo_lists
                .iter()
                .zip(list_areas.iter().copied())
                .enumerate()
            {
                let is_list_selected = i == todo_list_idx;
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

    fn set_mode(&mut self, next_mode: Mode) {
        if next_mode == Mode::Insert {
            self.create_snapshot();
        }
        match next_mode {
            Mode::Insert => self.set_mode_insert(),
            Mode::Normal => self.set_mode_normal(),
        }
    }

    fn set_mode_insert(&mut self) {
        let todo_list = &self.todo_lists[self.selection.todo_list];
        if todo_list.todos.is_empty() { return }
        self.selection.char = 0;
        self.mode = Mode::Insert;
    }

    fn set_mode_normal(&mut self) {
        self.mode = Mode::Normal;
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else { return };
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = &mut todo_list.todos[todo_idx];
        if todo.name.trim().is_empty() {
            todo_list.todos.remove(todo_idx);
            self.snapshots.pop_back();
        }
        if self.selection.todo > 0 {
            self.selection.todo -= 1;
        }
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

    fn move_up_half(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        let next_todo_idx = if todo_idx > MOVE_HALF_AMOUNT {
            todo_idx - MOVE_HALF_AMOUNT
        }
        else {
            0
        };
        self.select_todo(todo_list_idx, next_todo_idx);
    }

    fn move_down_half(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        let todo_list = &self.todo_lists[todo_list_idx];
        let last_todo_idx = match todo_list.todos.len() {
            0 => return,
            len => len-1,
        };
        let next_todo_idx = (todo_idx + MOVE_HALF_AMOUNT).min(last_todo_idx);
        self.select_todo(todo_list_idx, next_todo_idx);
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
        self.create_snapshot();
        self.set_mode_insert();
        let todo_list = &mut self.todo_lists[self.selection.todo_list];
        let todos = &mut todo_list.todos;
        let todo_idx = match below {
            false => self.selection.todo.min(todos.len()),
            true => (self.selection.todo + 1).min(todos.len()),
        };
        todos.insert(todo_idx, Todo::new(""));
        self.selection.todo = todo_idx;
        self.needs_saving = true;
    }
    
    fn toggle_mark(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else { return };
        self.create_snapshot();
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = &mut todo_list.todos[todo_idx];
        todo.marked = !todo.marked;
        self.needs_saving = true;
    }

    /// Removes the currently selected [`Todo`]
    fn delete_todo(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else { return };
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = &mut todo_list.todos[todo_idx];
        if !todo.marked {
            self.create_snapshot();
            let todo_list = &mut self.todo_lists[todo_list_idx];
            todo_list.todos.remove(todo_idx);
            self.needs_saving = true;
        }
        else if todo_list_idx != BACKLOG_LIST_IDX {
            self.create_snapshot();
            let todo_list = &mut self.todo_lists[todo_list_idx];
            let todo = todo_list.todos.remove(todo_idx);
            let backlog_todo_list = &mut self.todo_lists[BACKLOG_LIST_IDX];
            backlog_todo_list.todos.push(todo);
            self.needs_saving = true;
        }
    }

    fn move_todo_left(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_list_idx == 0 {
            return;
        };
        self.create_snapshot();
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = todo_list.todos.remove(todo_idx);
        let next_todo_list = &mut self.todo_lists[todo_list_idx - 1];
        let next_todo_idx = self.selection.todo.min(next_todo_list.todos.len());
        next_todo_list.todos.insert(next_todo_idx, todo);
        self.selection.todo_list -= 1;
        self.needs_saving = true;
    }

    fn move_todo_right(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_list_idx == self.todo_lists.len() - 1 {
            return;
        };
        self.create_snapshot();
        let todo_list = &mut self.todo_lists[todo_list_idx];
        let todo = todo_list.todos.remove(todo_idx);
        let next_todo_list = &mut self.todo_lists[todo_list_idx + 1];
        let next_todo_idx = self.selection.todo.min(next_todo_list.todos.len());
        next_todo_list.todos.insert(next_todo_idx, todo);
        self.selection.todo_list += 1;
        self.needs_saving = true;
    }

    fn move_todo_up(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        if todo_idx == 0 {
            return;
        };
        self.create_snapshot();
        let todo_list = &mut self.todo_lists[todo_list_idx];
        todo_list.todos.swap(todo_idx, todo_idx - 1);
        self.select_todo(todo_list_idx, todo_idx - 1);
        self.needs_saving = true;
    }

    fn move_todo_down(&mut self) {
        let Some((todo_list_idx, todo_idx)) = self.selected_todo() else {
            return;
        };
        let todo_list = &self.todo_lists[todo_list_idx];
        if todo_idx == todo_list.todos.len() - 1 {
            return;
        };
        self.create_snapshot();
        let todo_list = &mut self.todo_lists[todo_list_idx];
        todo_list.todos.swap(todo_idx, todo_idx + 1);
        self.select_todo(todo_list_idx, todo_idx + 1);
        self.needs_saving = true;
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
        self.needs_saving = true;
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

    fn save(&mut self) -> anyhow::Result<()> {
        if !self.needs_saving {
            return Ok(());
        }
        let dbpath = Path::new(&self.config.dbpath);
        if let Some(parent) = dbpath.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let state = State::create(self);
        let state_str = serde_yaml::to_string(&state)?;
        std::fs::write(dbpath, state_str)?;
        self.needs_saving = false;
        Ok(())
    }

    fn undo(&mut self) {
        if self.current_snapshot == 0 { return };
        self.current_snapshot -= 1;
        let mut snapshot = State::create(self);
        std::mem::swap(&mut snapshot, &mut self.snapshots[self.current_snapshot]);
        snapshot.restore(self);
        self.needs_saving = true;
    }

    fn redo(&mut self) {
        if self.current_snapshot == self.snapshots.len() { return };
        let mut snapshot = State::create(self);
        std::mem::swap(&mut snapshot, &mut self.snapshots[self.current_snapshot]);
        snapshot.restore(self);
        self.current_snapshot += 1;
        self.needs_saving = true;
    }

    fn quit(&mut self) -> anyhow::Result<()> {
        self.save()?;
        self.quit = true;
        Ok(())
    }

    fn create_snapshot(&mut self) {
        for i in (self.current_snapshot..self.snapshots.len()).rev() {
            self.snapshots.remove(i);
        }
        self.snapshots.push_back(State::create(self));
        self.current_snapshot += 1;
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
            self.current_snapshot -= 1;
        }
    }
}

/// Current item being selected in the [`App`].
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
struct Selection {
    todo_list: usize, // Todo list selected
    todo: usize,      // Todo in todo list selected
    char: usize,      // Index of character in todo selected, if any
}

/// Configures an [App].
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
struct Config {
    /// Todo-list dabase path.
    dbpath: String,
}

/// Subset of the fields in [`App`], which are saved to a database file.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
struct State {
    version: String,
    todo_lists: Vec<TodoList>,
}

impl State {
    fn create(app: &App) -> Self {
        Self {
            todo_lists: app.todo_lists.clone(),
            ..Default::default()
        }
    }

    fn restore(self, app: &mut App) {
        app.todo_lists = self.todo_lists;
    }
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

/// Default key mapping for various actions.
fn default_key_mappings() -> HashMap<KeyPress, Action> {
    let mut res = HashMap::new();
    res.insert(KeyPress::char(Mode::Normal, 'q'),                                       Action::Quit);
    res.insert(KeyPress::char(Mode::Normal, 'o'),                                       Action::AddTodoBelow);
    res.insert(KeyPress::char(Mode::Normal, 'O'),                                       Action::AddTodoAbove);
    res.insert(KeyPress::char(Mode::Normal, 'm'),                                       Action::ToggleMark);
    res.insert(KeyPress::char(Mode::Normal, 'd'),                                       Action::DeleteTodo);
    res.insert(KeyPress::char(Mode::Normal, 'H'),                                       Action::MoveTodoLeft);
    res.insert(KeyPress::char(Mode::Normal, 'J'),                                       Action::MoveTodoDown);
    res.insert(KeyPress::char(Mode::Normal, 'K'),                                       Action::MoveTodoUp);
    res.insert(KeyPress::char(Mode::Normal, 'L'),                                       Action::MoveTodoRight);
    res.insert(KeyPress::new(Mode::Normal, KeyCode::Left, KeyModifiers::SHIFT),         Action::MoveTodoLeft);
    res.insert(KeyPress::new(Mode::Normal, KeyCode::Down, KeyModifiers::SHIFT),         Action::MoveTodoDown);
    res.insert(KeyPress::new(Mode::Normal, KeyCode::Up, KeyModifiers::SHIFT),           Action::MoveTodoUp);
    res.insert(KeyPress::new(Mode::Normal, KeyCode::Right, KeyModifiers::SHIFT),        Action::MoveTodoRight);
    res.insert(KeyPress::char(Mode::Normal, 'K'),                                       Action::MoveTodoUp);
    res.insert(KeyPress::char(Mode::Normal, 'L'),                                       Action::MoveTodoRight);
    res.insert(KeyPress::char(Mode::Normal, 'h'),                                       Action::MoveLeft);
    res.insert(KeyPress::char(Mode::Normal, 'j'),                                       Action::MoveDown);
    res.insert(KeyPress::char(Mode::Normal, 'k'),                                       Action::MoveUp);
    res.insert(KeyPress::new(Mode::Normal, KeyCode::Char('d'), KeyModifiers::CONTROL),  Action::MoveDownHalf);
    res.insert(KeyPress::new(Mode::Normal, KeyCode::Char('u'), KeyModifiers::CONTROL),  Action::MoveUpHalf);
    res.insert(KeyPress::char(Mode::Normal, 'k'),                                       Action::MoveUp);
    res.insert(KeyPress::char(Mode::Normal, 'l'),                                       Action::MoveRight);
    res.insert(KeyPress::char(Mode::Normal, 'g'),                                       Action::MoveTop);
    res.insert(KeyPress::char(Mode::Normal, 'G'),                                       Action::MoveBottom);
    res.insert(KeyPress::code(Mode::Normal, KeyCode::Home),                             Action::MoveTop);
    res.insert(KeyPress::code(Mode::Normal, KeyCode::End),                              Action::MoveBottom);
    res.insert(KeyPress::code(Mode::Normal, KeyCode::Left),                             Action::MoveLeft);
    res.insert(KeyPress::code(Mode::Normal, KeyCode::Down),                             Action::MoveDown);
    res.insert(KeyPress::code(Mode::Normal, KeyCode::Up),                               Action::MoveUp);
    res.insert(KeyPress::code(Mode::Normal, KeyCode::Right),                            Action::MoveRight);
    res.insert(KeyPress::char(Mode::Normal, 'u'),                                       Action::Undo);
    res.insert(KeyPress::char(Mode::Normal, 'r'),                                       Action::Redo);
    res.insert(KeyPress::char(Mode::Normal, 'i'),                                       Action::SetMode(Mode::Insert));
    res.insert(KeyPress::code(Mode::Insert, KeyCode::Esc),                              Action::SetMode(Mode::Normal));
    res.insert(KeyPress::code(Mode::Insert, KeyCode::Right),                            Action::MoveCursorRight);
    res.insert(KeyPress::code(Mode::Insert, KeyCode::Left),                             Action::MoveCursorLeft);
    res.insert(KeyPress::code(Mode::Insert, KeyCode::Home),                             Action::MoveCursorStart);
    res.insert(KeyPress::code(Mode::Insert, KeyCode::End),                              Action::MoveCursorEnd);
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
    Quit,
    DeleteTodo,
    MoveTodoLeft,
    MoveTodoRight,
    MoveTodoUp,
    MoveTodoDown,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    MoveUpHalf,
    MoveDownHalf,
    MoveTop,
    MoveBottom,
    AddTodoAbove,
    AddTodoBelow,
    ToggleMark,
    Input(KeyCode),
    SetMode(Mode),
    MoveCursorRight,
    MoveCursorLeft,
    MoveCursorStart,
    MoveCursorEnd,
    Undo,
    Redo,
    Nop, // No operation. Useful if app needs to rerender.
}

/// Current mode of an [`App`] which determines the action keys map to.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum Mode {
    /// Initial mode, allowing user to navigate and move todo lists.
    Normal,
    /// Mode when inserting a value in the cell of a todo.
    Insert,
}

/// Represents a key press, while in a particular mode, with optional modifiers like shift and ctrl
/// being pressed.
#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub(crate) struct KeyPress {
    mode: Mode,
    code: KeyCode,
    modifiers: KeyModifiers, 
}

impl KeyPress {

    pub fn new(mode: Mode, code: KeyCode, modifiers: KeyModifiers) -> Self {
        let modifiers = match code {
            KeyCode::Char(c) if c.is_ascii_uppercase() => modifiers | KeyModifiers::SHIFT,
            _ => modifiers,
        };
        Self { mode, code, modifiers }
    }

    pub fn char(mode: Mode, char: char) -> Self {
        Self::new(mode, KeyCode::Char(char), KeyModifiers::empty())
    }

    pub fn code(mode: Mode, code: KeyCode) -> Self {
        Self::new(mode, code, KeyModifiers::empty())
    }
}
