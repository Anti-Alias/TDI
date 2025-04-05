use tdi::{App, TodoList};

fn main() {
    let mut app = App::default();
    app.push_todo_list(TodoList::new("Todo"));
    app.push_todo_list(TodoList::new("Backlog"));

    let terminal = ratatui::init();
    if let Err(err) = app.run(terminal) {
        eprintln!("{err}");
    }
    ratatui::restore();
}
