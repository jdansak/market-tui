// src/main.rs
//! Application entry point for Bloomberg Terminal TUI workstation.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::panic;
use std::time::Duration;

mod app;
mod correlation;
mod data;
mod insider;
mod options;
mod ticker;
mod timeframe;
mod ui;
mod yield_curve;

use app::App;
use ui::draw_ui;

#[tokio::main]
async fn main() -> Result<()> {
    // Set panic hook to ensure terminal is restored if panic occurs
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        default_hook(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();

    // Fetch initial active ticker data, quotes, and institutional analytics
    app.refresh_active().await.ok();
    app.refresh_all_quotes().await;

    // Event loop
    while !app.should_quit {
        terminal.draw(|f| draw_ui(f, &app))?;

        if event::poll(Duration::from_millis(80))? {
            let ev = event::read()?;
            if let Event::Key(key) = &ev {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
            }
            app.handle_event(ev).await.ok();
        }

        // Auto-refresh active ticker quote and chart every 20 seconds
        if app.last_refresh.elapsed() > Duration::from_secs(20) {
            app.refresh_active().await.ok();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;
    terminal.show_cursor()?;

    Ok(())
}
