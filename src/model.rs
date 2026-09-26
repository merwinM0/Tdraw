use crate::connection::Connection;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ShapeKind {
    #[default]
    Rectangle,
    Ellipse,
    Diamond,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Shape {
    #[serde(default)]
    pub(crate) shape: ShapeKind,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) connections: Vec<Connection>,
}

impl Shape {
    pub(crate) fn valid(&self) -> bool {
        [self.x, self.y, self.z, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.x >= 0.0
            && self.y >= 0.0
            && self.width >= 1.0
            && self.height >= 1.0
    }

    pub(crate) fn fit_text(&mut self) {
        let width = self
            .text
            .split('\n')
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0);
        let lines = self.text.split('\n').count();
        let (w, h) = match self.shape {
            ShapeKind::Rectangle => (width + 2, lines + 2),
            // The central third is safely inside both shapes even after raster
            // rounding. Padding keeps wide glyphs and the caret off the outline.
            _ => (3 * (width + 2) + 1, 3 * (lines + 2) + 1),
        };
        self.width = self.width.max(w as f64);
        self.height = self.height.max(h as f64);
    }

    pub(crate) fn center(&self) -> (f64, f64) {
        let center = (
            self.x.floor() + (self.width.floor() - 1.0) / 2.0,
            self.y.floor() + (self.height.floor() - 1.0) / 2.0,
        );
        if self.shape == ShapeKind::Rectangle {
            center
        } else {
            (center.0.round(), center.1.round())
        }
    }

    pub(crate) fn row_span(&self, y: f64) -> Option<(f64, f64)> {
        let top = self.y.floor();
        let h = self.height.floor() - 1.0;
        if y < top || y > top + h {
            return None;
        }
        let (cx, cy) = self.center();
        let radius = if y < cy { cy - top } else { top + h - cy };
        let dy = (y - cy).abs() / radius.max(1.0);
        let factor = match self.shape {
            ShapeKind::Rectangle => 1.0,
            ShapeKind::Ellipse => (1.0 - dy * dy).max(0.0).sqrt(),
            ShapeKind::Diamond => 1.0 - dy,
        };
        let left_radius = cx - self.x.floor();
        let right_radius = self.x.floor() + self.width.floor() - 1.0 - cx;
        Some((
            (cx - left_radius * factor).round(),
            (cx + right_radius * factor).round(),
        ))
    }

    pub(crate) fn column_span(&self, x: f64) -> Option<(f64, f64)> {
        let cy = self.center().1.round();
        if !self.contains(x, cy) {
            return None;
        }
        // Binary search the exact raster boundary, not a separately rounded ellipse.
        let (mut low, mut high) = (self.y.floor(), cy);
        while high - low > 1.0 {
            let mid = (low + (high - low) / 2.0).floor();
            if mid <= low || mid >= high {
                break;
            }
            if self.contains(x, mid) {
                high = mid;
            } else {
                low = mid;
            }
        }
        let top = if self.contains(x, low) { low } else { high };
        let (mut low, mut high) = (cy, self.y.floor() + self.height.floor() - 1.0);
        while high - low > 1.0 {
            let mid = (low + (high - low) / 2.0).floor();
            if mid <= low || mid >= high {
                break;
            }
            if self.contains(x, mid) {
                low = mid;
            } else {
                high = mid;
            }
        }
        Some((top, if self.contains(x, high) { high } else { low }))
    }

    pub(crate) fn on_border(&self, x: f64, y: f64) -> bool {
        self.contains(x, y)
            && [(x - 1.0, y), (x + 1.0, y), (x, y - 1.0), (x, y + 1.0)]
                .iter()
                .any(|&(px, py)| !self.contains(px, py))
    }

    pub(crate) fn text_region(&self) -> (f64, f64, f64, f64) {
        let (px, py) = if self.shape == ShapeKind::Rectangle {
            (1.0, 1.0)
        } else {
            (
                (self.width / 3.0).floor() + 1.0,
                (self.height / 3.0).floor() + 1.0,
            )
        };
        (
            self.x.floor() + px,
            self.y.floor() + py,
            (self.width.floor() - 2.0 * px).max(0.0),
            (self.height.floor() - 2.0 * py).max(0.0),
        )
    }

    pub(crate) fn contains(&self, x: f64, y: f64) -> bool {
        self.row_span(y)
            .is_some_and(|(left, right)| x >= left && x <= right)
    }
}
