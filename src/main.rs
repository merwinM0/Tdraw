use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    widgets::canvas::Canvas,
    widgets::{Block, Borders},
};
use std::{error::Error, io, time::Duration};

struct App {
    dot_x: f64,
    dot_y: f64,
}

impl App {
    fn new() -> App {
        App {
            dot_x: 20.0,
            dot_y: 10.0,
        }
    }
    fn move_dot(&mut self, dx: f64, dy: f64) {
        self.dot_x += dx;
        self.dot_y += dy;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| {
            let area = f.area(); // 顺便把 size 改成了更现代的 area

            let canvas = Canvas::default()
                .block(
                    Block::default()
                        .title(" Tdraw - WASD移动, Q退出 ")
                        .borders(Borders::ALL),
                )
                .x_bounds([0.0, area.width as f64])
                .y_bounds([0.0, area.height as f64])
                .paint(|ctx| {
                    ctx.print(app.dot_x, app.dot_y, "●");
                });

            f.render_widget(canvas, area);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                // 只有按键按下时才处理逻辑
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('w') => app.move_dot(0.0, 1.0),
                        KeyCode::Char('s') => app.move_dot(0.0, -1.0),
                        KeyCode::Char('a') => app.move_dot(-2.0, 0.0),
                        KeyCode::Char('d') => app.move_dot(2.0, 0.0),
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
