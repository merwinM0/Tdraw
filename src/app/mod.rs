mod input;
#[cfg(test)]
mod tests;

use crate::storage;
use crate::{
    connection::{self, Anchor},
    model::Rectangle,
};
use std::{io, path::PathBuf};

#[derive(Debug, Clone)]
pub(crate) enum Mode {
    Normal,
    SelectedConnection {
        source: usize,
        index: usize,
    },
    Connecting {
        source: usize,
        anchor: Anchor,
        arrow: bool,
        waypoints: Vec<(f64, f64)>,
    },
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

pub(crate) struct App {
    pub(crate) rectangles: Vec<Rectangle>,
    pub(crate) cursor: (f64, f64),
    pub(crate) size: (u16, u16),
    pub(crate) mode: Mode,
    pub(crate) path: PathBuf,
    pub(crate) message: String,
    pub(crate) dirty: bool,
    pub(crate) quit: bool,
}

impl App {
    pub(crate) fn load(path: PathBuf, size: (u16, u16)) -> io::Result<Self> {
        let rectangles = storage::load(&path)?;
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

    pub(crate) fn save(&mut self) -> bool {
        let result = storage::save(&self.path, &self.rectangles);
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

    pub(crate) fn resize(&mut self, width: u16, height: u16) {
        self.size = (width, height);
        self.cursor.0 = self.cursor.0.clamp(0.0, width.saturating_sub(1) as f64);
        self.cursor.1 = self.cursor.1.clamp(0.0, height.saturating_sub(1) as f64);
    }

    pub(crate) fn hovered(&self) -> Option<usize> {
        self.rectangles
            .iter()
            .enumerate()
            .filter(|(_, r)| r.contains(self.cursor.0, self.cursor.1))
            .max_by(|(ai, a), (bi, b)| a.z.total_cmp(&b.z).then(ai.cmp(bi)))
            .map(|(i, _)| i)
    }

    pub(crate) fn hovered_connection(&self) -> Option<(usize, usize)> {
        // Boxes are opaque and rendered above connections; prefer them at shared endpoints.
        if self.hovered().is_some() {
            return None;
        }
        // Reverse the drawing order so the visible topmost connection wins at crossings.
        self.rectangles
            .iter()
            .enumerate()
            .rev()
            .find_map(|(source, r)| {
                r.connections
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(index, c)| {
                        connection::contains(r, &self.rectangles[c.target], c, self.cursor)
                            .then_some((source, index))
                    })
            })
    }

    pub(crate) fn boundary(&self) -> Option<(usize, Anchor)> {
        // A hidden lower border must not be selected through an opaque upper box.
        let index = self.hovered()?;
        Anchor::at(&self.rectangles[index], self.cursor).map(|anchor| (index, anchor))
    }

    pub(crate) fn preview(&self, anchor: (f64, f64)) -> Rectangle {
        Rectangle {
            text: String::new(),
            connections: Vec::new(),
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

    pub(crate) fn cancel(&mut self) {
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

    pub(crate) fn commit(&mut self) {
        self.mode = Mode::Normal;
        self.dirty = true;
        self.save();
    }
}
