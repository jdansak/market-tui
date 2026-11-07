// src/main.rs
//! Application entry point for Bloomberg Terminal TUI workstation with continuous background pre-cache.

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
use tokio::sync::mpsc;

mod app;
mod cache;
mod correlation;
mod data;
mod insider;
mod options;
mod ticker;
mod timeframe;
mod ui;
mod yield_curve;

use app::App;
use cache::{start_background_worker, WorkerRequest};
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

    // Communication channels between App and background pre-cache worker
    let (tx_req, rx_req) = mpsc::unbounded_channel();
    let (tx_update, mut rx_update) = mpsc::unbounded_channel();

    let mut app = App::with_channels(tx_req.clone());

    // Launch continuous background worker for non-blocking pre-cache polling
    start_background_worker(app.watchlist.clone(), rx_req, tx_update);

    // Eagerly trigger active selected ticker pre-fetch
    if let Some(t) = app.selected_ticker().cloned() {
        let _ = tx_req.send(WorkerRequest::PrioritizeTicker {
            ticker: t,
            timeframe: app.timeframe,
        });
    }

    // High-performance event loop: sub-millisecond keyboard responsiveness + smooth cache rendering
    while !app.should_quit {
        terminal.draw(|f| draw_ui(f, &app))?;

        // 1. Snappy keyboard input handling (20ms poll, returns immediately when a key is pressed)
        if event::poll(Duration::from_millis(20))? {
            let ev = event::read()?;
            if let Event::Key(key) = &ev {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
            }
            app.handle_event(ev).ok();
        }

        // 2. Non-blocking drain of background pre-cache updates
        while let Ok(update) = rx_update.try_recv() {
            app.apply_update(update);
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::cursor::Show)?;
    terminal.show_cursor()?;

    Ok(())
}
