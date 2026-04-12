use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::Color,
    widgets::canvas::{Canvas, Rectangle},
    widgets::{Block, Borders, Paragraph},
    text::{Text, Line, Span},
    layout::{Alignment, Rect},
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::{error::Error, io, time::Duration};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct MyRect {
    x: f64,
    y: f64,
    z: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, PartialEq)]
enum AppMode {
    Normal,
    Drawing,
    Moving,
    Menu,
}

struct App {
    dot_x: f64,
    dot_y: f64,
    mode: AppMode,
    start_x: Option<f64>,
    start_y: Option<f64>,
    rects: Vec<MyRect>,
    selected_idx: Option<usize>,
    menu_items: Vec<String>,
    menu_selected: usize,
    menu_open: bool,
}

impl App {
    fn new(terminal_width: u16, terminal_height: u16) -> App {
        // 光标初始位置在终端中央
        let dot_x = terminal_width as f64 / 2.0;
        let dot_y = terminal_height as f64 / 2.0;
        
        App {
            dot_x,
            dot_y,
            mode: AppMode::Normal,
            start_x: None,
            start_y: None,
            rects: Vec::new(),
            selected_idx: None,
            menu_items: vec![
                "上移一层".to_string(),
                "下移一层".to_string(),
                "水平翻转".to_string(),
                "垂直翻转".to_string(),
            ],
            menu_selected: 0,
            menu_open: false,
        }
    }

    fn save_to_file(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(&self.rects)?;
        let mut file = File::create("rects.json")?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    fn load_from_file() -> Vec<MyRect> {
        if let Ok(mut file) = File::open("rects.json") {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                return serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new());
            }
        }
        Vec::new()
    }

    fn get_hovered_rect(&self) -> Option<usize> {
        let mut hovered_with_z: Vec<(usize, f64)> = self.rects.iter()
            .enumerate()
            .filter(|(_, r)| r.contains(self.dot_x, self.dot_y))
            .map(|(idx, r)| (idx, r.z))
            .collect();
        
        hovered_with_z.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        hovered_with_z.first().map(|(idx, _)| *idx)
    }

    fn move_dot(&mut self, dx: f64, dy: f64, width: u16, height: u16) {
        self.dot_x += dx;
        self.dot_y += dy;
        
        // 限制在画布范围内，根据终端大小自适应
        // 留出1个单位的边距，避免光标太靠近边缘
        self.dot_x = self.dot_x.max(1.0).min(width as f64 - 2.0);
        self.dot_y = self.dot_y.max(1.0).min(height as f64 - 2.0);
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

    // 获取初始终端大小
    let initial_size = terminal.size()?;
    let mut app = App::new(initial_size.width, initial_size.height);
    app.rects = App::load_from_file();

    loop {
        // 获取终端大小
        let terminal_size = terminal.size()?;
        let width = terminal_size.width;
        let height = terminal_size.height;
        
        terminal.draw(|f| {
            let area = f.area();

            // 绘制白色背景 - 占满整个终端
            let background_block = Block::default()
                .style(ratatui::style::Style::default().bg(Color::Rgb(255, 255, 255)));
            f.render_widget(background_block, area);

            // 绘制画布 - 无边框，占满整个区域
            let canvas = Canvas::default()
                .background_color(Color::Rgb(255, 255, 255))
                .x_bounds([0.0, width as f64])
                .y_bounds([0.0, height as f64])
                .paint(|ctx| {
                    // 绘制所有矩形
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
                        
                        // 绘制矩形边框
                        let line_width = 0.1;
                        // 上边框
                        ctx.draw(&Rectangle {
                            x: r.x,
                            y: r.y,
                            width: r.width,
                            height: line_width,
                            color: border_color,
                        });
                        // 下边框
                        ctx.draw(&Rectangle {
                            x: r.x,
                            y: r.y + r.height - line_width,
                            width: r.width,
                            height: line_width,
                            color: border_color,
                        });
                        // 左边框
                        ctx.draw(&Rectangle {
                            x: r.x,
                            y: r.y,
                            width: line_width,
                            height: r.height,
                            color: border_color,
                        });
                        // 右边框
                        ctx.draw(&Rectangle {
                            x: r.x + r.width - line_width,
                            y: r.y,
                            width: line_width,
                            height: r.height,
                            color: border_color,
                        });
                    }

                    // 绘制正在绘制的矩形
                    if let (Some(sx), Some(sy)) = (app.start_x, app.start_y) {
                        let draw_x = sx.min(app.dot_x);
                        let draw_y = sy.min(app.dot_y);
                        let draw_width = (app.dot_x - sx).abs();
                        let draw_height = (app.dot_y - sy).abs();
                        
                        let line_width = 0.1;
                        let color = Color::Rgb(0, 128, 0); // 绿色表示正在绘制
                        
                        // 上边框
                        ctx.draw(&Rectangle {
                            x: draw_x,
                            y: draw_y,
                            width: draw_width,
                            height: line_width,
                            color,
                        });
                        // 下边框
                        ctx.draw(&Rectangle {
                            x: draw_x,
                            y: draw_y + draw_height - line_width,
                            width: draw_width,
                            height: line_width,
                            color,
                        });
                        // 左边框
                        ctx.draw(&Rectangle {
                            x: draw_x,
                            y: draw_y,
                            width: line_width,
                            height: draw_height,
                            color,
                        });
                        // 右边框
                        ctx.draw(&Rectangle {
                            x: draw_x + draw_width - line_width,
                            y: draw_y,
                            width: line_width,
                            height: draw_height,
                            color,
                        });
                    }

                    // 绘制光标
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

            // 绘制菜单（如果打开）
            if app.menu_open {
                let menu_width = 20;
                let menu_height = app.menu_items.len() as u16 + 2;
                let menu_x = (area.width.saturating_sub(menu_width)) / 2;
                let menu_y = (area.height.saturating_sub(menu_height)) / 2;
                
                let menu_area = Rect::new(menu_x, menu_y, menu_width, menu_height);
                
                let menu_block = Block::default()
                    .title("菜单")
                    .borders(Borders::ALL)
                    .border_style(ratatui::style::Style::default().fg(Color::Rgb(0, 0, 255)));
                
                let mut menu_text = Text::default();
                for (i, item) in app.menu_items.iter().enumerate() {
                    let style = if i == app.menu_selected {
                        ratatui::style::Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(0, 0, 255))
                    } else {
                        ratatui::style::Style::default().fg(Color::Rgb(0, 0, 0))
                    };
                    menu_text.push_line(Line::from(Span::styled(item.clone(), style)));
                }
                
                let menu_paragraph = Paragraph::new(menu_text)
                    .block(menu_block)
                    .alignment(Alignment::Center);
                
                f.render_widget(menu_paragraph, menu_area);
            }

            // 绘制状态信息
            let status_text = match app.mode {
                AppMode::Normal => "正常模式 - 使用 WASD 移动，Enter 选择/取消选择",
                AppMode::Drawing => "绘制模式 - 使用 WASD 调整大小，Enter 确认",
                AppMode::Moving => "移动模式 - 使用 WASD 移动选中区块，Enter 确认",
                AppMode::Menu => "菜单模式 - 使用 WS 选择，Enter 确认，Q 退出",
            };
            
            let status_paragraph = Paragraph::new(Text::from(status_text))
                .style(ratatui::style::Style::default().fg(Color::Rgb(0, 0, 0)).bg(Color::Rgb(200, 200, 200)))
                .alignment(Alignment::Center);
            
            let status_area = Rect::new(0, area.height - 1, area.width, 1);
            f.render_widget(status_paragraph, status_area);
        })?;

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        // Ctrl+Q 退出程序
                        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.save_to_file()?;
                            break;
                        }
                        
                        // Ctrl+D 开始绘制
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if app.mode == AppMode::Normal && !app.menu_open {
                                app.mode = AppMode::Drawing;
                                app.start_x = Some(app.dot_x);
                                app.start_y = Some(app.dot_y);
                            }
                        }
                        
                        // Backspace 删除选中区块
                        KeyCode::Backspace => {
                            if let Some(idx) = app.selected_idx {
                                if app.mode == AppMode::Normal && !app.menu_open {
                                    app.rects.remove(idx);
                                    app.selected_idx = None;
                                }
                            }
                        }
                        
                        // M 打开菜单
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            if app.selected_idx.is_some() && app.mode == AppMode::Normal && !app.menu_open {
                                app.menu_open = true;
                                app.menu_selected = 0;
                            }
                        }
                        
                        // Q 键处理
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if app.menu_open {
                                app.menu_open = false;
                            } else if app.mode == AppMode::Drawing {
                                // 取消绘制
                                app.mode = AppMode::Normal;
                                app.start_x = None;
                                app.start_y = None;
                            } else if app.selected_idx.is_some() {
                                // 取消选择
                                app.selected_idx = None;
                            }
                        }
                        
                        // Enter 键处理
                        KeyCode::Enter => {
                            if app.menu_open {
                                // 菜单确认（暂时只显示效果，不实现功能）
                                app.menu_open = false;
                            } else if app.mode == AppMode::Drawing {
                                // 完成绘制
                                if let (Some(sx), Some(sy)) = (app.start_x, app.start_y) {
                                    let new_rect = MyRect {
                                        x: sx.min(app.dot_x),
                                        y: sy.min(app.dot_y),
                                        z: app.rects.len() as f64,
                                        width: (app.dot_x - sx).abs(),
                                        height: (app.dot_y - sy).abs(),
                                    };
                                    app.rects.push(new_rect);
                                    app.mode = AppMode::Normal;
                                    app.start_x = None;
                                    app.start_y = None;
                                }
                            } else if app.mode == AppMode::Moving {
                                // 完成移动
                                app.mode = AppMode::Normal;
                            } else if app.selected_idx.is_some() {
                                // 取消选择
                                app.selected_idx = None;
                            } else {
                                // 选择矩形
                                if let Some(idx) = app.get_hovered_rect() {
                                    app.selected_idx = Some(idx);
                                    app.mode = AppMode::Moving;
                                }
                            }
                        }
                        
                        // WASD 移动处理
                        KeyCode::Char('w') | KeyCode::Char('W') => {
                            if app.menu_open {
                                // 菜单中向上选择
                                if app.menu_selected > 0 {
                                    app.menu_selected -= 1;
                                }
                            } else if app.mode == AppMode::Drawing {
                                // 绘制时调整大小
                                app.move_dot(0.0, -1.0, width, height);
                            } else if app.mode == AppMode::Moving {
                                // 移动选中区块
                                if let Some(idx) = app.selected_idx {
                                    app.rects[idx].y -= 1.0;
                                }
                                app.move_dot(0.0, -1.0, width, height);
                            } else if app.mode == AppMode::Normal {
                                // 正常移动光标
                                app.move_dot(0.0, -1.0, width, height);
                            }
                        }
                        
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            if app.menu_open {
                                // 菜单中向下选择
                                if app.menu_selected < app.menu_items.len() - 1 {
                                    app.menu_selected += 1;
                                }
                            } else if app.mode == AppMode::Drawing {
                                // 绘制时调整大小
                                app.move_dot(0.0, 1.0, width, height);
                            } else if app.mode == AppMode::Moving {
                                // 移动选中区块
                                if let Some(idx) = app.selected_idx {
                                    app.rects[idx].y += 1.0;
                                }
                                app.move_dot(0.0, 1.0, width, height);
                            } else if app.mode == AppMode::Normal {
                                // 正常移动光标
                                app.move_dot(0.0, 1.0, width, height);
                            }
                        }
                        
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            if !app.menu_open {
                                if app.mode == AppMode::Drawing {
                                    // 绘制时调整大小
                                    app.move_dot(-2.0, 0.0, width, height);
                                } else if app.mode == AppMode::Moving {
                                    // 移动选中区块
                                    if let Some(idx) = app.selected_idx {
                                        app.rects[idx].x -= 2.0;
                                    }
                                    app.move_dot(-2.0, 0.0, width, height);
                                } else if app.mode == AppMode::Normal {
                                    // 正常移动光标
                                    app.move_dot(-2.0, 0.0, width, height);
                                }
                            }
                        }
                        
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            if !app.menu_open {
                                if app.mode == AppMode::Drawing {
                                    // 绘制时调整大小
                                    app.move_dot(2.0, 0.0, width, height);
                                } else if app.mode == AppMode::Moving {
                                    // 移动选中区块
                                    if let Some(idx) = app.selected_idx {
                                        app.rects[idx].x += 2.0;
                                    }
                                    app.move_dot(2.0, 0.0, width, height);
                                } else if app.mode == AppMode::Normal {
                                    // 正常移动光标
                                    app.move_dot(2.0, 0.0, width, height);
                                }
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
