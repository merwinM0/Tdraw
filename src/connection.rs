use crate::model::Shape;
use ratatui::{
    Frame,
    style::{Color, Style},
};
use serde::{Deserialize, Serialize};

mod anchor;
pub use anchor::{Anchor, Side};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    pub target: usize,
    pub from: Anchor,
    pub to: Anchor,
    pub arrow: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waypoints: Vec<(f64, f64)>,
}

impl Connection {
    /// Arrows attach to the sides facing the adjacent route point. Plain lines
    /// retain their user-selected anchors. Stored anchors remain backward compatible.
    pub fn anchors(&self, source: &Shape, target: &Shape) -> (Anchor, Anchor) {
        if !self.arrow {
            return (self.from, self.to);
        }
        (
            Anchor::facing(
                source,
                self.waypoints.first().copied().unwrap_or(target.center()),
            ),
            Anchor::facing(
                target,
                self.waypoints.last().copied().unwrap_or(source.center()),
            ),
        )
    }
}

type Point = (f64, f64);

// Shared geometry keeps hit testing identical to the rendered path, including arrow tips.
fn route(
    source: (&Shape, Anchor),
    target: Point,
    to: Option<Anchor>,
    waypoints: &[Point],
    arrow: bool,
) -> Vec<Point> {
    let start = source.1.point(source.0);
    let a = source.1.outward(start);
    let b = to.map_or(target, |anchor| anchor.outward(target));
    let mut points = vec![start, a];
    let mut previous = a;
    for next in waypoints.iter().copied() {
        let mid = match source.1.side {
            Side::Left | Side::Right => (next.0, previous.1),
            Side::Top | Side::Bottom => (previous.0, next.1),
        };
        points.extend([mid, next]);
        previous = next;
    }
    if arrow && let Some(to) = to {
        let horizontal_start = matches!(source.1.side, Side::Left | Side::Right);
        let horizontal_end = matches!(to.side, Side::Left | Side::Right);
        if waypoints.is_empty() && horizontal_start == horizontal_end {
            if horizontal_start {
                let mx = ((previous.0 + b.0) / 2.0).round();
                points.extend([(mx, previous.1), (mx, b.1)]);
            } else {
                let my = ((previous.1 + b.1) / 2.0).round();
                points.extend([(previous.0, my), (b.0, my)]);
            }
        } else {
            // Approach perpendicular to the target normal before the final stub.
            points.push(if horizontal_end {
                (b.0, previous.1)
            } else {
                (previous.0, b.1)
            });
        }
    } else {
        points.push(match source.1.side {
            Side::Left | Side::Right => (b.0, previous.1),
            Side::Top | Side::Bottom => (previous.0, b.1),
        });
    }
    points.extend([b, target]);
    points
}

fn on_segment(p: Point, a: Point, b: Point) -> bool {
    (a.1 == b.1 && p.1 == a.1 && p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0))
        || (a.0 == b.0 && p.0 == a.0 && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1))
}

pub fn contains(source: &Shape, target: &Shape, c: &Connection, point: Point) -> bool {
    let (from, to) = c.anchors(source, target);
    route(
        (source, from),
        to.point(target),
        Some(to),
        &c.waypoints,
        c.arrow,
    )
    .windows(2)
    .any(|pair| on_segment(point, pair[0], pair[1]))
}

mod render;

pub fn draw(
    f: &mut Frame,
    source: (&Shape, Anchor),
    target: (f64, f64),
    to: Option<Anchor>,
    arrow: bool,
    color: Color,
    waypoints: &[Point],
) {
    let points = route(source, target, to, waypoints, arrow);
    let b = points[points.len() - 2];
    let previous = points
        .iter()
        .rev()
        .copied()
        .find(|p| *p != target)
        .unwrap_or(target);
    render::path(f, &points, color);
    if arrow {
        // Arrowhead sits just outside the target border, pointing into the box.
        let tip = if to.is_some() { b } else { target };
        let symbol = match to.map(|a| a.side) {
            Some(Side::Left) => "▶",
            Some(Side::Right) => "◀",
            Some(Side::Top) => "▼",
            Some(Side::Bottom) => "▲",
            None if target.1 > previous.1 => "▼",
            None if target.1 < previous.1 => "▲",
            None if target.0 > previous.0 => "▶",
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
                .set_style(Style::default().fg(color));
        }
    }
}
