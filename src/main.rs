use tdi::{App, TodoList};

fn main() {
    let mut todo = TodoList::new("Todo");
    todo.push("Item");
    todo.push("Item");
    todo.push("Item");
    todo.push("Item");
    todo.push("Item");

    let mut backlog = TodoList::new("Backlog");
    backlog.push("Item");
    backlog.push("Item");
    backlog.push("Item");

    let mut finished = TodoList::new("Finished");
    finished.push("Item");
    finished.push("Item");

    let mut app = App::default();
    app.push_todo_list(todo);
    app.push_todo_list(backlog);
    app.push_todo_list(finished);

    let terminal = ratatui::init();
    if let Err(err) = app.run(terminal) {
        eprintln!("{err}");
    }
    ratatui::restore();
}
