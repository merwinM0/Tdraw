use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, stdout},
    path::{Path, PathBuf},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const INK: Color = Color::Rgb(35, 35, 42);
const PURPLE: Color = Color::Rgb(105, 65, 210);
const SOFT: Color = Color::Rgb(239, 234, 255);
const ITEMS: [&str; 4] = ["上移一层", "下移一层", "水平翻转", "垂直翻转"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Rectangle {
    x: f64,
    y: f64,
    z: f64,
    width: f64,
    height: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,
}

impl Rectangle {
    fn valid(&self) -> bool {
        [self.x, self.y, self.z, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width >= 1.0
            && self.height >= 1.0
    }

    fn fit_text(&mut self) {
        let width = self
            .text
            .split('\n')
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        self.width = self.width.max((width + 2) as f64);
        self.height = self.height.max((self.text.split('\n').count() + 2) as f64);
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

#[derive(Debug, Clone)]
enum Mode {
    Normal,
    Drawing {
        anchor: (f64, f64),
    },
    Moving {
        index: usize,
        original: Rectangle,
        cursor: (f64, f64),
    },
    Editing {
        index: usize,
        original: Rectangle,
        cursor: (f64, f64),
        before_edit: Rectangle,
    },
    Menu {
        index: usize,
        original: Rectangle,
        cursor: (f64, f64),
        item: usize,
    },
}

struct App {
    rectangles: Vec<Rectangle>,
    cursor: (f64, f64),
    size: (u16, u16),
    mode: Mode,
    path: PathBuf,
    message: String,
    dirty: bool,
    quit: bool,
}

impl App {
    fn load(path: PathBuf, size: (u16, u16)) -> io::Result<Self> {
        let rectangles: Vec<Rectangle> = match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).map_err(io::Error::other)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        if !rectangles.iter().all(Rectangle::valid) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "rects.json 包含无效坐标或尺寸",
            ));
        }
        Ok(Self {
            rectangles,
            cursor: ((size.0 / 2) as f64, (size.1 / 2) as f64),
            size,
            mode: Mode::Normal,
            path,
            message: "就绪 · 本地画布".into(),
            dirty: false,
            quit: false,
        })
    }

    fn save(&mut self) -> bool {
        let result = (|| -> io::Result<()> {
            let data = serde_json::to_vec_pretty(&self.rectangles).map_err(io::Error::other)?;
            let temp = self.path.with_extension("json.tmp");
            fs::write(&temp, data)?;
            fs::rename(temp, &self.path)
        })();
        match result {
            Ok(()) => {
                self.dirty = false;
                self.message = "已保存 · rects.json".into();
                true
            }
            Err(e) => {
                self.dirty = true;
                self.message = format!("保存失败: {e} · Ctrl+S 重试");
                false
            }
        }
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.size = (width, height);
        self.cursor.0 = self.cursor.0.clamp(0.0, width.saturating_sub(1) as f64);
        self.cursor.1 = self.cursor.1.clamp(0.0, height.saturating_sub(1) as f64);
    }

    fn hovered(&self) -> Option<usize> {
        self.rectangles
            .iter()
            .enumerate()
            .filter(|(_, r)| r.contains(self.cursor.0, self.cursor.1))
            .max_by(|(ai, a), (bi, b)| a.z.total_cmp(&b.z).then(ai.cmp(bi)))
            .map(|(i, _)| i)
    }

    fn preview(&self, anchor: (f64, f64)) -> Rectangle {
        Rectangle {
            text: String::new(),
            x: anchor.0.min(self.cursor.0),
            y: anchor.1.min(self.cursor.1),
            width: (anchor.0 - self.cursor.0).abs() + 1.0,
            height: (anchor.1 - self.cursor.1).abs() + 1.0,
            z: self
                .rectangles
                .iter()
                .map(|r| r.z)
                .max_by(f64::total_cmp)
                .map_or(0.0, |z| z + 1.0),
        }
    }

    fn cancel(&mut self) {
        match self.mode.clone() {
            Mode::Editing {
                index,
                original,
                cursor,
                before_edit,
            } => {
                self.rectangles[index] = before_edit;
                self.mode = Mode::Moving {
                    index,
                    original,
                    cursor,
                };
                return;
            }
            Mode::Menu {
                index,
                original,
                cursor,
                ..
            } => {
                self.mode = Mode::Moving {
                    index,
                    original,
                    cursor,
                };
                return;
            }
            Mode::Moving {
                index,
                original,
                cursor,
            } => {
                self.rectangles[index] = original;
                self.cursor = cursor;
                self.resize(self.size.0, self.size.1);
            }
            _ => {}
        }
        self.mode = Mode::Normal;
    }

    fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') => {
                    if matches!(self.mode, Mode::Menu { .. } | Mode::Editing { .. }) {
                        self.cancel();
                    }
                    self.cancel();
                    if !self.dirty || self.save() {
                        self.quit = true;
                    }
                }
                KeyCode::Char('d') if matches!(self.mode, Mode::Normal) => {
                    self.mode = Mode::Drawing {
                        anchor: self.cursor,
                    };
                }
                KeyCode::Char('s') if matches!(self.mode, Mode::Normal) => {
                    self.save();
                }
                KeyCode::Char('j') if matches!(self.mode, Mode::Editing { .. }) => {
                    if let Mode::Editing { index, .. } = self.mode {
                        self.rectangles[index].text.push('\n');
                        self.rectangles[index].fit_text();
                    }
                }
                KeyCode::Char('z') => self.message = "撤回暂未启用".into(),
                _ => {}
            }
            return;
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            return;
        }
        if let Mode::Editing { index, .. } = self.mode {
            match key.code {
                KeyCode::Esc => self.cancel(),
                KeyCode::Enter => self.commit(),
                KeyCode::Backspace => {
                    let text = &mut self.rectangles[index].text;
                    if let Some((offset, _)) = text.grapheme_indices(true).next_back() {
                        text.truncate(offset);
                    }
                }
                KeyCode::Char(c) if !c.is_control() => {
                    self.rectangles[index].text.push(c);
                    self.rectangles[index].fit_text();
                }
                _ => {}
            }
            return;
        }
        if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
            self.cancel();
            return;
        }
        if let Mode::Menu { ref mut item, .. } = self.mode {
            match key.code {
                KeyCode::Char('w') | KeyCode::Up => *item = (*item + 3) % 4,
                KeyCode::Char('s') | KeyCode::Down => *item = (*item + 1) % 4,
                KeyCode::Enter => {
                    self.cancel();
                    self.message = "菜单仅作预览 · 尚未执行操作".into();
                }
                _ => {}
            }
            return;
        }
        let delta = match key.code {
            KeyCode::Char('w') | KeyCode::Up => Some((0.0, -1.0)),
            KeyCode::Char('a') | KeyCode::Left => Some((-1.0, 0.0)),
            KeyCode::Char('s') | KeyCode::Down => Some((0.0, 1.0)),
            KeyCode::Char('d') | KeyCode::Right => Some((1.0, 0.0)),
            _ => None,
        };
        if let Some((mut dx, mut dy)) = delta {
            dx = (self.cursor.0 + dx).clamp(0.0, self.size.0.saturating_sub(1) as f64)
                - self.cursor.0;
            dy = (self.cursor.1 + dy).clamp(0.0, self.size.1.saturating_sub(1) as f64)
                - self.cursor.1;
            if let Mode::Moving { index, .. } = self.mode {
                let r = &mut self.rectangles[index];
                dx = dx.max(-r.x);
                dy = dy.max(-r.y);
                r.x += dx;
                r.y += dy;
            }
            self.cursor.0 += dx;
            self.cursor.1 += dy;
            return;
        }
        match (key.code, self.mode.clone()) {
            (KeyCode::Enter, Mode::Normal) => {
                if let Some(index) = self.hovered() {
                    self.mode = Mode::Moving {
                        index,
                        original: self.rectangles[index].clone(),
                        cursor: self.cursor,
                    };
                }
            }
            (KeyCode::Enter, Mode::Drawing { anchor }) => {
                self.rectangles.push(self.preview(anchor));
                self.commit();
            }
            (KeyCode::Enter, Mode::Moving { .. }) => self.commit(),
            (
                KeyCode::Char('r'),
                Mode::Moving {
                    index,
                    original,
                    cursor,
                },
            ) => {
                self.mode = Mode::Editing {
                    index,
                    original,
                    cursor,
                    before_edit: self.rectangles[index].clone(),
                };
                self.rectangles[index].fit_text();
            }
            (KeyCode::Backspace | KeyCode::Delete, Mode::Moving { index, .. }) => {
                self.rectangles.remove(index);
                self.commit();
            }
            (
                KeyCode::Char('m'),
                Mode::Moving {
                    index,
                    original,
                    cursor,
                },
            ) => {
                self.mode = Mode::Menu {
                    index,
                    original,
                    cursor,
                    item: 0,
                };
            }
            _ => {}
        }
    }

    fn commit(&mut self) {
        self.mode = Mode::Normal;
        self.dirty = true;
        self.save();
    }
}

// Draw only visible cells: clipping must not manufacture borders at viewport edges.
fn draw_rectangle(f: &mut Frame, r: &Rectangle, color: Color) {
    let area = f.area();
    let left = r.x.floor();
    let top = r.y.floor();
    let right = left + r.width.floor() - 1.0;
    let bottom = top + r.height.floor() - 1.0;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let px = x as f64;
            let py = y as f64;
            if px < left || px > right || py < top || py > bottom {
                continue;
            }
            let symbol = if py == top && px == left {
                "┌"
            } else if py == top && px == right {
                "┐"
            } else if py == bottom && px == left {
                "└"
            } else if py == bottom && px == right {
                "┘"
            } else if py == top || py == bottom {
                "─"
            } else if px == left || px == right {
                "│"
            } else {
                " "
            };
            f.buffer_mut()[(x, y)]
                .set_symbol(symbol)
                .set_style(Style::default().fg(color).bg(Color::White));
        }
    }
}

fn draw_text(f: &mut Frame, r: &Rectangle) {
    let area = f.area();
    let left = r.x.floor() + 1.0;
    let top = r.y.floor() + 1.0;
    if left >= area.right() as f64 || top >= area.bottom() as f64 {
        return;
    }
    let width = (r.width.floor() - 2.0)
        .max(0.0)
        .min(area.right() as f64 - left) as u16;
    let height = (r.height.floor() - 2.0)
        .max(0.0)
        .min(area.bottom() as f64 - top) as u16;
    f.render_widget(
        Paragraph::new(r.text.as_str()).style(Style::default().fg(INK).bg(Color::White)),
        Rect::new(left as u16, top as u16, width, height),
    );
}

fn panel() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(205, 198, 225)))
        .style(Style::default().fg(INK).bg(Color::White))
}

fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Color::White).fg(INK)),
        area,
    );
    let selected = match app.mode {
        Mode::Moving { index, .. } | Mode::Menu { index, .. } | Mode::Editing { index, .. } => {
            Some(index)
        }
        _ => None,
    };
    let hovered = if matches!(app.mode, Mode::Normal) {
        app.hovered()
    } else {
        None
    };
    let mut order: Vec<_> = app.rectangles.iter().enumerate().collect();
    order.sort_by(|(ai, a), (bi, b)| a.z.total_cmp(&b.z).then(ai.cmp(bi)));
    for (i, r) in order {
        draw_rectangle(
            f,
            r,
            if selected == Some(i) {
                PURPLE
            } else if hovered == Some(i) {
                Color::Blue
            } else {
                INK
            },
        );
        draw_text(f, r);
    }
    if let Mode::Drawing { anchor } = app.mode {
        draw_rectangle(f, &app.preview(anchor), PURPLE);
    }
    let (mode, help) = match app.mode {
        Mode::Normal => ("选择", "WASD 移动 · Ctrl+D 矩形 · Enter 选择 · Ctrl+Q 退出"),
        Mode::Drawing { .. } => (
            "矩形",
            "WASD 调整对角点 · Enter 保存 · Q 取消 · Ctrl+Q 退出",
        ),
        Mode::Moving { .. } => (
            "移动",
            "WASD 移动 · R 文字 · Enter 保存 · Q 还原 · ⌫ 删除 · M 菜单",
        ),
        Mode::Editing { .. } => (
            "文字",
            "输入文字 · Ctrl+J 换行 · ⌫ 退格 · Enter 保存 · Esc 取消",
        ),
        Mode::Menu { .. } => ("菜单", "W/S 选择 · Enter 预览 · Q 返回 · Ctrl+Q 退出"),
    };
    if area.width >= 20 && area.height >= 8 {
        let width = area.width.saturating_sub(4).min(66);
        f.render_widget(
            Paragraph::new(format!(" Tdraw   │   ↖ 选择    ▣ 矩形 Ctrl+D   │   {mode}"))
                .block(panel())
                .style(Style::default().fg(PURPLE).bg(SOFT)),
            Rect::new((area.width - width) / 2, 0, width, 3),
        );
        f.render_widget(
            Paragraph::new(format!(
                " {help}\n {}  │  {} 区块  │  {},{}{}",
                app.message,
                app.rectangles.len(),
                app.cursor.0,
                app.cursor.1,
                if app.dirty { " · 未保存" } else { "" }
            ))
            .style(Style::default().fg(INK).bg(SOFT)),
            Rect::new(0, area.height - 2, area.width, 2),
        );
    }
    let (x, y) = (app.cursor.0 as u16, app.cursor.1 as u16);
    if let Mode::Editing { index, .. } = app.mode {
        let r = &app.rectangles[index];
        let tx = r.x.floor() + 1.0 + r.text.split('\n').next_back().unwrap_or("").width() as f64;
        let ty = r.y.floor() + r.text.split('\n').count() as f64;
        if tx < area.width as f64 && ty < area.height as f64 {
            f.set_cursor_position((tx as u16, ty as u16));
        }
    } else if x < area.width && y < area.height {
        f.render_widget(
            Paragraph::new("●").style(Style::default().fg(Color::Black).bg(Color::White)),
            Rect::new(x, y, 1, 1),
        );
    }
    if let Mode::Menu { item, .. } = app.mode {
        let width = area.width.min(26);
        let height = area.height.min(7);
        let mx = if x.saturating_add(2).saturating_add(width) <= area.width {
            x + 2
        } else {
            x.saturating_sub(width)
        };
        let my = y.min(area.height.saturating_sub(height));
        let popup = Rect::new(mx, my, width, height);
        f.render_widget(Clear, popup);
        f.render_widget(panel().title(" 区块操作 · 预览 "), popup);
        for (i, label) in ITEMS.iter().enumerate() {
            if i as u16 + 2 >= height {
                break;
            }
            f.render_widget(
                Paragraph::new(format!(" {} {label}", if i == item { "›" } else { " " })).style(
                    Style::default()
                        .fg(if i == item { PURPLE } else { INK })
                        .bg(if i == item { SOFT } else { Color::White }),
                ),
                Rect::new(mx + 1, my + 1 + i as u16, width.saturating_sub(2), 1),
            );
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn app() -> App {
        let path = std::env::temp_dir().join(format!(
            "tdraw-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        App::load(path, (80, 24)).unwrap()
    }
    fn key(app: &mut App, code: KeyCode) {
        app.key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    fn ctrl(app: &mut App, c: char) {
        app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
    }
    fn draw(app: &mut App) {
        ctrl(app, 'd');
        key(app, KeyCode::Char('a'));
        key(app, KeyCode::Char('w'));
        key(app, KeyCode::Enter);
    }
    #[test]
    fn drawing_preview_and_persistence() {
        let mut a = app();
        draw(&mut a);
        assert_eq!(
            a.rectangles[0],
            Rectangle {
                x: 39.0,
                y: 11.0,
                width: 2.0,
                height: 2.0,
                z: 0.0,
                text: String::new()
            }
        );
        assert_eq!(
            App::load(a.path.clone(), a.size).unwrap().rectangles,
            a.rectangles
        );
        fs::remove_file(a.path).unwrap();
    }
    #[test]
    fn move_cancel_menu_and_delete() {
        let mut a = app();
        draw(&mut a);
        let original = a.rectangles.clone();
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('d'));
        key(&mut a, KeyCode::Char('m'));
        let cursor = a.cursor;
        key(&mut a, KeyCode::Char('d'));
        key(&mut a, KeyCode::Char('w'));
        assert_eq!(a.cursor, cursor);
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('q'));
        assert_eq!(a.rectangles, original);
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Backspace);
        assert!(
            App::load(a.path.clone(), a.size)
                .unwrap()
                .rectangles
                .is_empty()
        );
        fs::remove_file(a.path).unwrap();
    }
    #[test]
    fn layers_and_bounds() {
        let mut a = app();
        draw(&mut a);
        a.cursor = (40.0, 12.0);
        draw(&mut a);
        assert_eq!(a.hovered(), Some(1));
        a.cursor = (41.0, 12.0);
        assert_eq!(a.hovered(), None);
        a.resize(1, 1);
        key(&mut a, KeyCode::Char('a'));
        assert_eq!(a.cursor, (0.0, 0.0));
        fs::remove_file(a.path).unwrap();
    }
    #[test]
    fn quit_cancels_unconfirmed_changes() {
        let mut a = app();
        draw(&mut a);
        let original = a.rectangles.clone();
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('d'));
        key(&mut a, KeyCode::Char('m'));
        ctrl(&mut a, 'q');
        assert!(a.quit);
        assert_eq!(a.rectangles, original);
        fs::remove_file(a.path).unwrap();
        let mut a = app();
        ctrl(&mut a, 'd');
        ctrl(&mut a, 'q');
        assert!(a.quit);
        assert!(!a.path.exists());
    }
    #[test]
    fn render_small_and_clipped_canvases() {
        let mut a = app();
        a.rectangles.push(Rectangle {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 100000.0,
            height: 100000.0,
            text: String::new(),
        });
        a.mode = Mode::Menu {
            index: 0,
            original: a.rectangles[0].clone(),
            cursor: a.cursor,
            item: 0,
        };
        for (w, h) in [(1, 1), (8, 4), (80, 24)] {
            a.resize(w, h);
            let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| ui(f, &a)).unwrap();
        }
    }
    #[test]
    fn text_editing_grows_saves_and_loads() {
        let mut a = app();
        draw(&mut a);
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('r'));
        for c in "中文qwasd".chars() {
            key(&mut a, KeyCode::Char(c));
        }
        ctrl(&mut a, 'j');
        key(&mut a, KeyCode::Char('好'));
        assert_eq!(a.rectangles[0].width, 11.0);
        assert_eq!(a.rectangles[0].height, 4.0);
        key(&mut a, KeyCode::Enter);
        let loaded = App::load(a.path.clone(), a.size).unwrap();
        assert_eq!(loaded.rectangles[0].text, "中文qwasd\n好");
        assert_eq!(loaded.rectangles, a.rectangles);
        fs::remove_file(a.path).unwrap();
    }
    #[test]
    fn text_cancel_backspace_and_quit() {
        let mut a = app();
        draw(&mut a);
        let original = a.rectangles.clone();
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('r'));
        for c in "e\u{301}".chars() {
            key(&mut a, KeyCode::Char(c));
        }
        key(&mut a, KeyCode::Backspace);
        assert!(a.rectangles[0].text.is_empty());
        key(&mut a, KeyCode::Esc);
        assert_eq!(a.rectangles, original);
        assert!(matches!(a.mode, Mode::Moving { .. }));
        key(&mut a, KeyCode::Char('r'));
        key(&mut a, KeyCode::Char('中'));
        ctrl(&mut a, 'q');
        assert!(a.quit);
        assert_eq!(a.rectangles, original);
        fs::remove_file(a.path).unwrap();
    }
    #[test]
    fn legacy_json_defaults_to_empty_text() {
        let r: Rectangle =
            serde_json::from_str(r#"{"x":0,"y":0,"z":0,"width":3,"height":3}"#).unwrap();
        assert!(r.text.is_empty());
    }
    #[test]
    fn invalid_file_is_preserved() {
        let a = app();
        fs::write(&a.path, "not json").unwrap();
        assert!(App::load(a.path.clone(), a.size).is_err());
        assert_eq!(fs::read_to_string(&a.path).unwrap(), "not json");
        fs::remove_file(a.path).unwrap();
    }
}
