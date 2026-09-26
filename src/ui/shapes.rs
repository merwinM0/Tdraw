use super::theme::INK;
use crate::model::{Shape, ShapeKind};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
};

// Draw only visible cells: clipping must not manufacture borders at viewport edges.
pub(super) fn draw_shape(f: &mut Frame, r: &Shape, color: Color) {
    let area = f.area();
    let left = r.x.floor();
    let top = r.y.floor();
    let right = left + r.width.floor() - 1.0;
    let bottom = top + r.height.floor() - 1.0;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let px = x as f64;
            let py = y as f64;
            if !r.contains(px, py) {
                continue;
            }
            let symbol = if r.shape != ShapeKind::Rectangle {
                if !r.on_border(px, py) {
                    " "
                } else {
                    let (cx, cy) = r.center();
                    if py == top || py == bottom {
                        if r.shape == ShapeKind::Diamond {
                            "◆"
                        } else {
                            "─"
                        }
                    } else if px == left || px == right {
                        if r.shape == ShapeKind::Diamond {
                            "◆"
                        } else {
                            "│"
                        }
                    } else if r.row_span(py).is_some_and(|(l, r)| px > l && px < r) {
                        "─"
                    } else if r.contains(px, py - 1.0) && r.contains(px, py + 1.0) {
                        "│"
                    } else if (px < cx) == (py < cy) {
                        "╱"
                    } else {
                        "╲"
                    }
                }
            } else if py == top && px == left {
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

pub(super) fn draw_text(f: &mut Frame, r: &Shape) {
    let area = f.area();
    let (left, top, inner_width, inner_height) = r.text_region();
    if left >= area.right() as f64 || top >= area.bottom() as f64 {
        return;
    }
    let width = inner_width.min(area.right() as f64 - left) as u16;
    let height = inner_height.min(area.bottom() as f64 - top) as u16;
    f.render_widget(
        Paragraph::new(r.text.as_str()).style(Style::default().fg(INK).bg(Color::White)),
        Rect::new(left as u16, top as u16, width, height),
    );
}
