use tdi::App;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
    }
}

fn run() -> anyhow::Result<()> {
    let app = App::init()?;
    let terminal = ratatui::init();
    if let Err(err) = app.run(terminal) {
        eprintln!("{err}");
    }
    ratatui::restore();
    Ok(())
}

