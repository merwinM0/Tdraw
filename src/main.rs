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
    selected_idx: Option<usize>,
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
            selected_idx: None,
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

            let background_block = Block::default()
                .style(ratatui::style::Style::default().bg(Color::Rgb(255, 255, 255)));
            f.render_widget(background_block, area);

            let canvas = Canvas::default()
                .block(
                    Block::default()
                        .title("Tdraw")
                        .borders(Borders::ALL)
                         .border_style(ratatui::style::Style::default().fg(Color::Rgb(0, 0, 0)))
                         .title_style(ratatui::style::Style::default().fg(Color::Rgb(0, 0, 0))),
                )
                .background_color(Color::Rgb(255, 255, 255))
                .x_bounds([0.0, area.width as f64])
                .y_bounds([0.0, area.height as f64])
                .paint(|ctx| {
                     // 计算悬停的矩形用于后续逻辑（如果需要）

                     for (idx, r) in app.rects.iter().enumerate() {
                         let is_selected = app.selected_idx == Some(idx);
                         let is_hovered = r.contains(app.dot_x, app.dot_y) && !is_selected;
                         
                         let border_color = if is_selected {
                             Color::Rgb(255, 0, 0) // 红色 - 选中
                         } else if is_hovered {
                             Color::Rgb(0, 0, 255) // 蓝色 - 悬停
                         } else {
                             Color::Rgb(0, 0, 0) // 黑色 - 普通
                         };
                         
                         // 绘制矩形边框（四条线）
                         // 上边框
                         ctx.draw(&Rectangle {
                             x: r.x,
                             y: r.y,
                             width: r.width,
                             height: 0.1,
                             color: border_color,
                         });
                         // 下边框
                         ctx.draw(&Rectangle {
                             x: r.x,
                             y: r.y + r.height - 0.1,
                             width: r.width,
                             height: 0.1,
                             color: border_color,
                         });
                         // 左边框
                         ctx.draw(&Rectangle {
                             x: r.x,
                             y: r.y,
                             width: 0.1,
                             height: r.height,
                             color: border_color,
                         });
                         // 右边框
                         ctx.draw(&Rectangle {
                             x: r.x + r.width - 0.1,
                             y: r.y,
                             width: 0.1,
                             height: r.height,
                             color: border_color,
                         });
                     }

                     if let (Some(sx), Some(sy)) = (app.start_x, app.start_y) {
                         // 绘制正在绘制的矩形边框
                         let draw_x = sx.min(app.dot_x);
                         let draw_y = sy.min(app.dot_y);
                         let draw_width = (app.dot_x - sx).abs();
                         let draw_height = (app.dot_y - sy).abs();
                         
                         // 上边框
                         ctx.draw(&Rectangle {
                             x: draw_x,
                             y: draw_y,
                             width: draw_width,
                             height: 0.1,
                             color: Color::Rgb(0, 0, 255),
                         });
                         // 下边框
                         ctx.draw(&Rectangle {
                             x: draw_x,
                             y: draw_y + draw_height - 0.1,
                             width: draw_width,
                             height: 0.1,
                             color: Color::Rgb(0, 0, 255),
                         });
                         // 左边框
                         ctx.draw(&Rectangle {
                             x: draw_x,
                             y: draw_y,
                             width: 0.1,
                             height: draw_height,
                             color: Color::Rgb(0, 0, 255),
                         });
                         // 右边框
                         ctx.draw(&Rectangle {
                             x: draw_x + draw_width - 0.1,
                             y: draw_y,
                             width: 0.1,
                             height: draw_height,
                             color: Color::Rgb(0, 0, 255),
                         });
                     }
                     ctx.print(
                         app.dot_x,
                         app.dot_y,
                         ratatui::text::Span::styled(
                             "●",
                             ratatui::style::Style::default().fg(Color::Rgb(0, 0, 0)),
                         ),
                     );
                });

            f.render_widget(canvas, area);
        })?;

        if event::poll(Duration::from_millis(10))? {
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

                        // 修改后的 WASD 逻辑
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            app.dot_y += 1.0;
                            if let Some(idx) = app.selected_idx {
                                app.rects[idx].y += 1.0;
                            }
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            app.dot_y -= 1.0;
                            if let Some(idx) = app.selected_idx {
                                app.rects[idx].y -= 1.0;
                            }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            app.dot_x -= 2.0;
                            if let Some(idx) = app.selected_idx {
                                app.rects[idx].x -= 2.0;
                            }
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            app.dot_x += 2.0;
                            if let Some(idx) = app.selected_idx {
                                app.rects[idx].x += 2.0;
                            }
                        }

                        // 修改后的 Enter 逻辑
                        KeyCode::Enter => {
                            if app.is_drawing {
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
                            } else if app.selected_idx.is_some() {
                                app.selected_idx = None;
                             } else {
                                 // 找到小点所在的最上层矩形
                                 let mut hovered_with_z: Vec<(usize, f64)> = app.rects.iter()
                                     .enumerate()
                                     .filter(|(_, r)| r.contains(app.dot_x, app.dot_y))
                                     .map(|(idx, r)| (idx, r.z))
                                     .collect();
                                 
                                 // 按 z 值降序排序，选择最上层的
                                 hovered_with_z.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                                 
                                 app.selected_idx = hovered_with_z.first().map(|(idx, _)| *idx);
                             }
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
