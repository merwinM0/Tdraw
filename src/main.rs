use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    widgets::{Block, BorderType, Borders},
};
use std::{
    error::Error,
    io::{self, Stdout},
};

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?; //开启原始模式，确保把输入传给rust程序
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let block = Block::default().title("画布").borders(Borders::ALL);
            f.render_widget(block, size);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
