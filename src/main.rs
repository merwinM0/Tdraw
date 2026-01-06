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
use std::{error::Error, io, time::Duration};

// 1. 定义区块数据结构
struct MyRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

// 2. 定义 App 状态机 (类似前端的 State)
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
}

fn main() -> Result<(), Box<dyn Error>> {
    // 终端初始化
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        // --- 绘图逻辑 (UI) ---
        terminal.draw(|f| {
            let area = f.area();

            let canvas = Canvas::default()
                .block(
                    Block::default()
                        .title(" Tdraw: WASD移动, Shift+方向键画框, Enter保存, Q退出 ")
                        .borders(Borders::ALL),
                )
                .x_bounds([0.0, area.width as f64])
                .y_bounds([0.0, area.height as f64])
                .paint(|ctx| {
                    // 1. 画出保存好的区块
                    for r in &app.rects {
                        ctx.draw(&Rectangle {
                            x: r.x,
                            y: r.y,
                            width: r.width,
                            height: r.height,
                            color: Color::White,
                        });
                    }

                    // 2. 如果正在画框，画出黄色预览框
                    if let (Some(sx), Some(sy)) = (app.start_x, app.start_y) {
                        ctx.draw(&Rectangle {
                            x: sx.min(app.dot_x),
                            y: sy.min(app.dot_y),
                            width: (app.dot_x - sx).abs(),
                            height: (app.dot_y - sy).abs(),
                            color: Color::Yellow,
                        });
                    }

                    // 3. 画出移动圆点
                    ctx.print(app.dot_x, app.dot_y, "●");
                }); // 这里需要分号！

            f.render_widget(canvas, area);
        })?;

        // --- 事件处理逻辑 (Interaction) ---
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                    match key.code {
                        KeyCode::Char('q') => break,

                        // Shift + 方向键：进入/保持画框模式
                        KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
                            if has_shift =>
                        {
                            if !app.is_drawing {
                                app.is_drawing = true;
                                app.start_x = Some(app.dot_x);
                                app.start_y = Some(app.dot_y);
                            }
                            match key.code {
                                KeyCode::Up => app.dot_y += 1.0,
                                KeyCode::Down => app.dot_y -= 1.0,
                                KeyCode::Left => app.dot_x -= 2.0,
                                KeyCode::Right => app.dot_x += 2.0,
                                _ => {}
                            }
                        }

                        // 普通 WASD：移动
                        KeyCode::Char('w') => app.dot_y += 1.0,
                        KeyCode::Char('s') => app.dot_y -= 1.0,
                        KeyCode::Char('a') => app.dot_x -= 2.0,
                        KeyCode::Char('d') => app.dot_x += 2.0,

                        // Enter：确认画框
                        KeyCode::Enter if app.is_drawing => {
                            let new_rect = MyRect {
                                x: app.start_x.unwrap().min(app.dot_x),
                                y: app.start_y.unwrap().min(app.dot_y),
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
