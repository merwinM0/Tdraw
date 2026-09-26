use crate::model::{Shape, ShapeKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub side: Side,
    pub offset: f64,
}

impl Anchor {
    pub fn at(r: &Shape, cursor: (f64, f64)) -> Option<Self> {
        let (x, y) = cursor;
        if r.shape != ShapeKind::Rectangle {
            if !r.on_border(x, y) {
                return None;
            }
            let mut anchor = Self::facing(r, cursor);
            anchor.offset = match anchor.side {
                Side::Left | Side::Right => (y - r.y.floor()) / (r.height.floor() - 1.0).max(1.0),
                Side::Top | Side::Bottom => (x - r.x.floor()) / (r.width.floor() - 1.0).max(1.0),
            };
            return Some(anchor);
        }
        let left = r.x.floor();
        let top = r.y.floor();
        let w = (r.width.floor() - 1.0).max(0.0);
        let h = (r.height.floor() - 1.0).max(0.0);
        if x < left || x > left + w || y < top || y > top + h {
            return None;
        }
        let (side, offset) = if x == left {
            (Side::Left, (y - top) / h.max(1.0))
        } else if x == left + w {
            (Side::Right, (y - top) / h.max(1.0))
        } else if y == top {
            (Side::Top, (x - left) / w.max(1.0))
        } else if y == top + h {
            (Side::Bottom, (x - left) / w.max(1.0))
        } else {
            return None;
        };
        Some(Self { side, offset })
    }
    pub fn valid(self) -> bool {
        self.offset.is_finite() && (0.0..=1.0).contains(&self.offset)
    }
    pub fn facing(r: &Shape, toward: (f64, f64)) -> Self {
        let center = r.center();
        let dx = (toward.0 - center.0) / r.width.max(1.0);
        let dy = (toward.1 - center.1) / r.height.max(1.0);
        let side = if dx.abs() >= dy.abs() {
            if dx >= 0.0 { Side::Right } else { Side::Left }
        } else if dy >= 0.0 {
            Side::Bottom
        } else {
            Side::Top
        };
        Self { side, offset: 0.5 }
    }

    pub fn point(self, r: &Shape) -> (f64, f64) {
        let w = (r.width.floor() - 1.0).max(0.0);
        let h = (r.height.floor() - 1.0).max(0.0);
        let (x, y) = match self.side {
            Side::Top => ((w * self.offset).round(), 0.0),
            Side::Bottom => ((w * self.offset).round(), h),
            Side::Left => (0.0, (h * self.offset).round()),
            Side::Right => (w, (h * self.offset).round()),
        };
        let (x, y) = (r.x.floor() + x, r.y.floor() + y);
        if r.shape == ShapeKind::Rectangle {
            return (x, y);
        }
        match self.side {
            Side::Left => (r.row_span(y).unwrap().0, y),
            Side::Right => (r.row_span(y).unwrap().1, y),
            Side::Top => (x, r.column_span(x).unwrap().0),
            Side::Bottom => (x, r.column_span(x).unwrap().1),
        }
    }
    pub(super) fn outward(self, point: (f64, f64)) -> (f64, f64) {
        let (x, y) = point;
        match self.side {
            Side::Top => (x, y - 1.0),
            Side::Bottom => (x, y + 1.0),
            Side::Left => (x - 1.0, y),
            Side::Right => (x + 1.0, y),
        }
    }
}
