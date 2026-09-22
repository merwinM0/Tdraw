use super::{INK, PURPLE, Rectangle};
use ratatui::{Frame, style::Style};
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
    pub fn at(r: &Rectangle, cursor: (f64, f64)) -> Option<Self> {
        let (x, y) = cursor;
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
    pub fn point(self, r: &Rectangle) -> (f64, f64) {
        let w = (r.width.floor() - 1.0).max(0.0);
        let h = (r.height.floor() - 1.0).max(0.0);
        let (x, y) = match self.side {
            Side::Top => ((w * self.offset).round(), 0.0),
            Side::Bottom => ((w * self.offset).round(), h),
            Side::Left => (0.0, (h * self.offset).round()),
            Side::Right => (w, (h * self.offset).round()),
        };
        (r.x.floor() + x, r.y.floor() + y)
    }
    fn outward(self, point: (f64, f64)) -> (f64, f64) {
        let (x, y) = point;
        match self.side {
            Side::Top => (x, y - 1.0),
            Side::Bottom => (x, y + 1.0),
            Side::Left => (x - 1.0, y),
            Side::Right => (x + 1.0, y),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub target: usize,
    pub from: Anchor,
    pub to: Anchor,
    pub arrow: bool,
}

// Orthogonal segments are clipped by iterating the viewport, never world-sized ranges.
fn segment(f: &mut Frame, a: (f64, f64), b: (f64, f64), preview: bool) {
    let area = f.area();
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let p = (x as f64, y as f64);
            let horizontal = a.1 == b.1 && p.1 == a.1 && p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0);
            let vertical = a.0 == b.0 && p.0 == a.0 && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1);
            if horizontal || vertical {
                let cell = &mut f.buffer_mut()[(x, y)];
                let symbol = if (horizontal && cell.symbol() == "│")
                    || (vertical && cell.symbol() == "─")
                {
                    "┼"
                } else if horizontal {
                    "─"
                } else {
                    "│"
                };
                cell.set_symbol(symbol)
                    .set_style(Style::default().fg(if preview { PURPLE } else { INK }));
            }
        }
    }
}

pub fn draw(
    f: &mut Frame,
    source: (&Rectangle, Anchor),
    target: (f64, f64),
    to: Option<Anchor>,
    arrow: bool,
    preview: bool,
) {
    let start = source.1.point(source.0);
    let a = source.1.outward(start);
    let b = to.map_or(target, |anchor| anchor.outward(target));
    let mid = match source.1.side {
        Side::Left | Side::Right => (b.0, a.1),
        Side::Top | Side::Bottom => (a.0, b.1),
    };
    for (p, q) in [(start, a), (a, mid), (mid, b), (b, target)] {
        segment(f, p, q, preview);
    }
    if arrow {
        // Arrowhead sits just outside the target border, pointing into the box.
        let tip = if to.is_some() { b } else { target };
        let symbol = match to.map(|a| a.side) {
            Some(Side::Left) => "▶",
            Some(Side::Right) => "◀",
            Some(Side::Top) => "▼",
            Some(Side::Bottom) => "▲",
            None if target.0 > start.0 => "▶",
            None => "◀",
        };
        let area = f.area();
        if tip.0 >= 0.0
            && tip.1 >= 0.0
            && tip.0 < area.right() as f64
            && tip.1 < area.bottom() as f64
        {
            f.buffer_mut()[(tip.0 as u16, tip.1 as u16)]
                .set_symbol(symbol)
                .set_style(Style::default().fg(if preview { PURPLE } else { INK }));
        }
    }
}
