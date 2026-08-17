mod common;
mod db;
mod encryption;
mod hostkey;
mod ssh;
mod term;
mod ui;

use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::crossterm::execute;
use std::io::stdout;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use ui::{App, EventSink, UiEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let _ = db::init_db();
    encryption::EncryptionManager::init();

    // Runtime for the SSH manager's async tasks.
    let rt = tokio::runtime::Runtime::new()?;
    let _guard = rt.enter();

    let manager = Arc::new(ssh::SshManager::default());

    let (tx, rx) = mpsc::unbounded_channel::<UiEvent>();
    let sink = Arc::new(EventSink::new(tx.clone()));

    let mut terminal = ratatui::init();
    let result = run_tui(&mut terminal, manager, sink, tx, rx);
    ratatui::restore();

    if let Err(e) = &result {
        eprintln!("error: {}", e);
    }
    result
}

fn run_tui(
    terminal: &mut ratatui::DefaultTerminal,
    manager: Arc<ssh::SshManager>,
    sink: Arc<EventSink>,
    tx: mpsc::UnboundedSender<UiEvent>,
    rx: mpsc::UnboundedReceiver<UiEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new(manager.clone(), sink, tx, rx);

    // Enable bracketed paste so crossterm surfaces Event::Paste, plus mouse
    // capture (passively ignored for now, harmless).
    let _ = execute!(
        stdout(),
        event::EnableBracketedPaste,
        event::EnableMouseCapture
    );

    loop {
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press
                        || key.kind == KeyEventKind::Repeat
                    {
                        handle_key(&mut app, key);
                    }
                }
                Event::Paste(text) => {
                    handle_paste(&mut app, text);
                }
                Event::Resize(_, _) => {
                    // The next draw re-syncs the terminal pane size.
                }
                Event::Mouse(_) => {}
                _ => {}
            }
        }

        app.on_tick();
        app.drain_events();

        terminal.draw(|frame| app.draw(frame))?;

        if app.quit {
            break;
        }
    }

    app.shutdown();
    let _ = execute!(
        stdout(),
        event::DisableMouseCapture,
        event::DisableBracketedPaste
    );
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    app.handle_key(key);
}

fn handle_paste(app: &mut App, text: String) {
    app.paste(text);
}