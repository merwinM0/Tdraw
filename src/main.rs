use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs,
    io::{self, stdout},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Rectangle {
    x: f64,
    y: f64,
    z: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
enum AppMode {
    Normal,
    Drawing(Rectangle),
    Moving(usize),
    Menu(usize, usize), // (selected rectangle index, selected menu item)
}

#[derive(Debug)]
struct App {
    cursor_x: f64,
    cursor_y: f64,
    rectangles: Vec<Rectangle>,
    mode: AppMode,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            cursor_x: 40.0, // Start in the middle
            cursor_y: 12.0, // Start in the middle
            rectangles: Vec::new(),
            mode: AppMode::Normal,
            should_quit: false,
        }
    }

    fn load_rectangles(&mut self) -> Result<(), Box<dyn Error>> {
        if Path::new("rects.json").exists() {
            let data = fs::read_to_string("rects.json")?;
            self.rectangles = serde_json::from_str(&data)?;
        }
        Ok(())
    }

    fn save_rectangles(&self) -> Result<(), Box<dyn Error>> {
        let data = serde_json::to_string_pretty(&self.rectangles)?;
        fs::write("rects.json", data)?;
        Ok(())
    }

    fn move_cursor(&mut self, dx: f64, dy: f64) {
        self.cursor_x += dx;
        self.cursor_y += dy;
    }

    fn start_drawing(&mut self) {
        let rect = Rectangle {
            x: self.cursor_x,
            y: self.cursor_y,
            z: 0.0,
            width: 1.0,
            height: 1.0,
        };
        self.mode = AppMode::Drawing(rect);
    }

    fn update_drawing(&mut self, dx: f64, dy: f64) {
        if let AppMode::Drawing(ref mut rect) = self.mode {
            rect.width += dx;
            rect.height += dy;
        }
    }

    fn finish_drawing(&mut self) {
        if let AppMode::Drawing(rect) = &self.mode {
            let mut new_rect = rect.clone();
            // Ensure width and height are positive
            if new_rect.width < 0.0 {
                new_rect.x += new_rect.width;
                new_rect.width = -new_rect.width;
            }
            if new_rect.height < 0.0 {
                new_rect.y += new_rect.height;
                new_rect.height = -new_rect.height;
            }
            
            // Find highest z value and set new rectangle to be on top
            let max_z = self.rectangles.iter()
                .map(|r| r.z)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
            new_rect.z = max_z + 1.0;
            
            self.rectangles.push(new_rect);
            self.mode = AppMode::Normal;
        }
    }

    fn find_rectangle_at_cursor(&self) -> Option<usize> {
        // Sort by z value (highest first) to check top rectangles first
        let mut indices: Vec<usize> = (0..self.rectangles.len()).collect();
        indices.sort_by(|&a, &b| {
            self.rectangles[b].z.partial_cmp(&self.rectangles[a].z).unwrap()
        });
        
        for &i in &indices {
            let rect = &self.rectangles[i];
            if self.cursor_x >= rect.x && self.cursor_x <= rect.x + rect.width &&
               self.cursor_y >= rect.y && self.cursor_y <= rect.y + rect.height {
                return Some(i);
            }
        }
        None
    }

    fn select_rectangle(&mut self) {
        if let Some(index) = self.find_rectangle_at_cursor() {
            self.mode = AppMode::Moving(index);
        }
    }

    fn move_selected_rectangle(&mut self, dx: f64, dy: f64) {
        if let AppMode::Moving(index) = self.mode {
            if let Some(rect) = self.rectangles.get_mut(index) {
                rect.x += dx;
                rect.y += dy;
            }
        }
    }

    fn finish_moving(&mut self) {
        self.mode = AppMode::Normal;
    }

    fn delete_selected_rectangle(&mut self) {
        if let AppMode::Moving(index) = self.mode {
            if index < self.rectangles.len() {
                self.rectangles.remove(index);
                self.mode = AppMode::Normal;
            }
        }
    }

    fn open_menu(&mut self) {
        if let AppMode::Moving(index) = self.mode {
            self.mode = AppMode::Menu(index, 0);
        }
    }

    fn navigate_menu(&mut self, up: bool) {
        if let AppMode::Menu(_index, ref mut selection) = self.mode {
            let menu_items = 4; // 上移一层，下移一层，水平翻转，垂直翻转
            if up {
                *selection = (*selection + menu_items - 1) % menu_items;
            } else {
                *selection = (*selection + 1) % menu_items;
            }
        }
    }

    fn close_menu(&mut self) {
        if let AppMode::Menu(index, _) = self.mode {
            self.mode = AppMode::Moving(index);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key),
            AppMode::Drawing(_) => self.handle_drawing_mode(key),
            AppMode::Moving(_) => self.handle_moving_mode(key),
            AppMode::Menu(_, _) => self.handle_menu_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.start_drawing();
            }
            KeyCode::Char('w') => self.move_cursor(0.0, -1.0),
            KeyCode::Char('a') => self.move_cursor(-1.0, 0.0),
            KeyCode::Char('s') => self.move_cursor(0.0, 1.0),
            KeyCode::Char('d') => self.move_cursor(1.0, 0.0),
            KeyCode::Enter => self.select_rectangle(),
            _ => {}
        }
    }

    fn handle_drawing_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('w') => self.update_drawing(0.0, -1.0),
            KeyCode::Char('a') => self.update_drawing(-1.0, 0.0),
            KeyCode::Char('s') => self.update_drawing(0.0, 1.0),
            KeyCode::Char('d') => self.update_drawing(1.0, 0.0),
            KeyCode::Enter => self.finish_drawing(),
            KeyCode::Char('q') => self.mode = AppMode::Normal,
            _ => {}
        }
    }

    fn handle_moving_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('w') => self.move_selected_rectangle(0.0, -1.0),
            KeyCode::Char('a') => self.move_selected_rectangle(-1.0, 0.0),
            KeyCode::Char('s') => self.move_selected_rectangle(0.0, 1.0),
            KeyCode::Char('d') => self.move_selected_rectangle(1.0, 0.0),
            KeyCode::Enter => self.finish_moving(),
            KeyCode::Backspace => self.delete_selected_rectangle(),
            KeyCode::Char('m') => self.open_menu(),
            KeyCode::Char('q') => self.mode = AppMode::Normal,
            _ => {}
        }
    }

    fn handle_menu_mode(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('w') => self.navigate_menu(true),
            KeyCode::Char('s') => self.navigate_menu(false),
            KeyCode::Enter => self.close_menu(),
            KeyCode::Char('q') => self.close_menu(),
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    app.load_rectangles()?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    // Save rectangles on exit
    app.save_rectangles()?;

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app)).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key);
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();
    
    // Draw white canvas
    let canvas = Block::default()
        .style(Style::default().bg(Color::White));
    f.render_widget(canvas, size);
    
    // Draw rectangles
    let mut sorted_rectangles: Vec<(usize, &Rectangle)> = app.rectangles.iter().enumerate().collect();
    sorted_rectangles.sort_by(|(_, a), (_, b)| a.z.partial_cmp(&b.z).unwrap());
    
    for (index, rect) in sorted_rectangles {
        let rect_area = Rect::new(
            rect.x as u16,
            rect.y as u16,
            rect.width.max(1.0) as u16,
            rect.height.max(1.0) as u16,
        );
        
        let is_selected = match app.mode {
            AppMode::Moving(selected_idx) => selected_idx == index,
            AppMode::Menu(selected_idx, _) => selected_idx == index,
            _ => false,
        };
        
        let is_hovered = app.find_rectangle_at_cursor() == Some(index) && matches!(app.mode, AppMode::Normal);
        
        let border_color = if is_selected {
            Color::Red
        } else if is_hovered {
            Color::Blue
        } else {
            Color::Black
        };
        
        let rect_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        
        f.render_widget(rect_block, rect_area);
        
        // Draw z value in the center of rectangle
        let center_x = rect.x as u16 + rect.width as u16 / 2;
        let center_y = rect.y as u16 + rect.height as u16 / 2;
        if center_x < size.width && center_y < size.height {
            let z_text = Paragraph::new(format!("z: {:.0}", rect.z))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Black));
            f.render_widget(z_text, Rect::new(center_x.saturating_sub(5), center_y, 10, 1));
        }
    }
    
    // Draw cursor
    let cursor_x = app.cursor_x as u16;
    let cursor_y = app.cursor_y as u16;
    if cursor_x < size.width && cursor_y < size.height {
        let cursor = Paragraph::new("●")
            .style(Style::default().fg(Color::Black));
        f.render_widget(cursor, Rect::new(cursor_x, cursor_y, 1, 1));
    }
    
    // Draw mode indicator
    let mode_text = match app.mode {
        AppMode::Normal => "Normal Mode".to_string(),
        AppMode::Drawing(_) => "Drawing Mode - Use WASD to resize, Enter to confirm, q to cancel".to_string(),
        AppMode::Moving(_) => "Moving Mode - Use WASD to move, Enter to confirm, Backspace to delete, m for menu, q to cancel".to_string(),
        AppMode::Menu(_, selection) => {
            let menu_items = ["上移一层", "下移一层", "水平翻转", "垂直翻转"];
            format!("Menu - {} (Use WS to navigate, Enter/q to close)", menu_items[selection])
        }
    };
    
    let mode_indicator = Paragraph::new(mode_text)
        .style(Style::default().fg(Color::Black).bg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    
    let mode_area = Rect::new(0, 0, size.width.min(80), 3);
    f.render_widget(mode_indicator, mode_area);
    
    // Draw help text
    let help_text = match app.mode {
        AppMode::Normal => "Ctrl+D: Draw | Ctrl+Q: Quit | WASD: Move cursor | Enter: Select",
        AppMode::Drawing(_) => "WASD: Resize rectangle | Enter: Confirm | Q: Cancel",
        AppMode::Moving(_) => "WASD: Move rectangle | Enter: Confirm | Backspace: Delete | M: Menu | Q: Cancel",
        AppMode::Menu(_, _) => "W/S: Navigate menu | Enter/Q: Close menu",
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Black).bg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    
    let help_area = Rect::new(0, size.height.saturating_sub(3), size.width, 3);
    f.render_widget(help, help_area);
}