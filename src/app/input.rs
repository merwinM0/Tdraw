use super::{App, Mode};
use crate::{connection::Connection, model::ShapeKind};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

impl App {
    pub(crate) fn key(&mut self, key: KeyEvent) {
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
                KeyCode::Char(c @ ('d' | 'w' | 'a')) if matches!(self.mode, Mode::Normal) => {
                    self.mode = Mode::Drawing {
                        anchor: self.cursor,
                        shape: match c {
                            'w' => ShapeKind::Ellipse,
                            'a' => ShapeKind::Diamond,
                            _ => ShapeKind::Rectangle,
                        },
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
        if let Mode::Connecting {
            source,
            anchor,
            arrow,
            mut waypoints,
        } = self.mode.clone()
        {
            if key.code == KeyCode::Enter
                || key.code == KeyCode::Char(if arrow { 'k' } else { 'l' })
            {
                if let Some((target, to)) = self.boundary().filter(|(i, _)| *i != source) {
                    self.rectangles[source].connections.push(Connection {
                        target,
                        from: anchor,
                        to,
                        arrow,
                        waypoints,
                    });
                    self.commit();
                } else if key.code != KeyCode::Enter {
                    if waypoints.last().copied() != Some(self.cursor) {
                        waypoints.push(self.cursor);
                    }
                    self.message =
                        format!("已记录 {} 个中间点 · 继续移动并确认终点", waypoints.len());
                    self.mode = Mode::Connecting {
                        source,
                        anchor,
                        arrow,
                        waypoints,
                    };
                } else {
                    self.message = "请移至另一方框边界确认，或按 L/K 记录中间点".into();
                }
            }
            return;
        }
        match (key.code, self.mode.clone()) {
            (KeyCode::Char(c @ ('l' | 'k')), Mode::Normal) => {
                if let Some((source, anchor)) = self.boundary() {
                    self.mode = Mode::Connecting {
                        source,
                        anchor,
                        arrow: c == 'k',
                        waypoints: Vec::new(),
                    };
                    self.message = "已固定起点 · 移至另一个方框边界确认".into();
                } else {
                    self.message = "请先将光标移至方框的可见边界".into();
                }
            }
            (KeyCode::Enter, Mode::Normal) => {
                if let Some(index) = self.hovered() {
                    self.mode = Mode::Moving {
                        index,
                        original: self.rectangles[index].clone(),
                        cursor: self.cursor,
                    };
                } else if let Some((source, index)) = self.hovered_connection() {
                    self.mode = Mode::SelectedConnection { source, index };
                }
            }
            (KeyCode::Backspace | KeyCode::Delete, Mode::SelectedConnection { source, index }) => {
                self.rectangles[source].connections.remove(index);
                self.commit();
            }
            (KeyCode::Enter, Mode::SelectedConnection { .. }) => self.mode = Mode::Normal,
            (KeyCode::Enter, Mode::Drawing { anchor, .. }) => {
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
                for r in &mut self.rectangles {
                    r.connections.retain(|c| c.target != index);
                    for c in &mut r.connections {
                        if c.target > index {
                            c.target -= 1;
                        }
                    }
                }
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
}
