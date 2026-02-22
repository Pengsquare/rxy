mod app;
mod categories;
mod config;
mod feed;
mod paper_faves;
mod read_state;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>>
{
    if std::env::args().any(|a| a == "--version" || a == "-V")
    {
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    // Kick off the initial load in the background — UI is immediately responsive.
    app.load_feed();

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result
    {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()>
{
    loop
    {
        app.tick();

        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(100))?
        {
            if let Event::Key(key) = event::read()?
            {
                if key.kind == KeyEventKind::Press
                {
                    if app.on_key(key.code)
                    {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
