use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::Color, // 修复 Color 报错
    widgets::canvas::{Canvas, Rectangle},
    widgets::{Block, Borders},
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::{error::Error, io, time::Duration};

#[derive(Serialize, Deserialize, Clone)]
struct MyRect {
    x: f64,
    y: f64,
    z: f64,
    width: f64,
    height: f64,
}

struct App {
    dot_x: f64,
    dot_y: f64,
    is_drawing: bool,
    start_x: Option<f64>,
    start_y: Option<f64>,
    rects: Vec<MyRect>,
}

impl App {
    fn new() -> App {
        App {
            dot_x: 20.0,
            dot_y: 10.0,
            is_drawing: false,
            start_x: None,
            start_y: None,
            rects: Vec::new(),
        }
    }

    fn save_to_file(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(&self.rects)?; // 类似 JSON.stringify
        let mut file = File::create("rects.json")?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> Vec<MyRect> {
        if let Ok(mut file) = File::open("rects.json") {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                // 类似 JSON.parse，如果解析失败则返回空数组
                return serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new());
            }
        }
        Vec::new()
    }
}

impl MyRect {
    fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.rects = App::load_from_file();

    loop {
        terminal.draw(|f| {
            let area = f.area();

            let canvas = Canvas::default()
                .block(Block::default().title("Tdraw").borders(Borders::ALL))
                .x_bounds([0.0, area.width as f64])
                .y_bounds([0.0, area.height as f64])
                .paint(|ctx| {
                    let top_selected_z = app
                        .rects
                        .iter()
                        .filter(|r| r.contains(app.dot_x, app.dot_y))
                        .map(|r| r.z)
                        .fold(f64::NEG_INFINITY, f64::max);

                    for r in &app.rects {
                        let is_selected = (r.z - top_selected_z).abs() < f64::EPSILON;
                        ctx.draw(&Rectangle {
                            x: r.x,
                            y: r.y,
                            width: r.width,
                            height: r.height,
                            color: if is_selected {
                                Color::Red
                            } else {
                                Color::White
                            },
                        });
                    }

                    if let (Some(sx), Some(sy)) = (app.start_x, app.start_y) {
                        ctx.draw(&Rectangle {
                            x: sx.min(app.dot_x),
                            y: sy.min(app.dot_y),
                            width: (app.dot_x - sx).abs(),
                            height: (app.dot_y - sy).abs(),
                            color: Color::Yellow,
                        });
                    }

                    ctx.print(app.dot_x, app.dot_y, "●");
                });

            f.render_widget(canvas, area);
        })?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                    match key.code {
                        KeyCode::Char('q') => {
                            app.save_to_file()?;
                            break;
                        }
                        KeyCode::Char('z') | KeyCode::Char('Z') => {
                            app.rects.pop();
                        }

                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            app.rects.clear();
                        }

                        KeyCode::Up
                        | KeyCode::Down
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('d')
                        | KeyCode::Char('D')
                            if has_shift =>
                        {
                            if !app.is_drawing {
                                app.is_drawing = true;
                                app.start_x = Some(app.dot_x);
                                app.start_y = Some(app.dot_y);
                            }
                            match key.code {
                                // KeyCode::Char('j') | KeyCode::Char('J') => {
                                //     app.dot_x += 1.0;
                                //     app.dot_y -= 1.0;
                                // }
                                // KeyCode::Char('k') | KeyCode::Char('K') => {
                                //     app.dot_x -= 1.0;
                                //     app.dot_y += 1.0;
                                // }
                                KeyCode::Up => app.dot_y += 1.0,
                                KeyCode::Down => app.dot_y -= 1.0,
                                KeyCode::Left => app.dot_x -= 2.0,
                                KeyCode::Right => app.dot_x += 2.0,
                                _ => {}
                            }
                        }

                        KeyCode::Char('w') => app.dot_y += 1.0,
                        KeyCode::Char('s') => app.dot_y -= 1.0,
                        KeyCode::Char('a') => app.dot_x -= 2.0,
                        KeyCode::Char('d') => app.dot_x += 2.0,

                        KeyCode::Enter if app.is_drawing => {
                            let new_rect = MyRect {
                                x: app.start_x.unwrap().min(app.dot_x),
                                y: app.start_y.unwrap().min(app.dot_y),
                                z: app.rects.len() as f64,
                                width: (app.dot_x - app.start_x.unwrap()).abs(),
                                height: (app.dot_y - app.start_y.unwrap()).abs(),
                            };
                            app.rects.push(new_rect);
                            app.is_drawing = false;
                            app.start_x = None;
                            app.start_y = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // 恢复终端
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
