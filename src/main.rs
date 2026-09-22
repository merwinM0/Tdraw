mod app;
mod connection;
mod model;
mod storage;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use std::{
    io::{self, stdout},
    path::Path,
};
use ui::ui;

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
    }
}

fn main() -> io::Result<()> {
    // Validate before taking over the terminal; malformed files are never overwritten.
    let mut app = App::load(Path::new("rects.json").into(), crossterm::terminal::size()?)?;
    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        old_hook(info);
    }));
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;
    while !app.quit {
        terminal.draw(|f| ui(f, &app))?;
        match event::read()? {
            Event::Key(key) => app.key(key),
            Event::Resize(w, h) => app.resize(w, h),
            _ => {}
        }
    }
    Ok(())
}
