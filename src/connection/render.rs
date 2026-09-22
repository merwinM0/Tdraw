//! Rasterize orthogonal paths by joining their actual incident directions.
//! A bend has two directions (rounded corner), not four (a false crossing).
use super::Point;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
};
use std::collections::BTreeMap;

const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;
type Cells = BTreeMap<(u16, u16), u8>;

fn symbol(mask: u8) -> &'static str {
    match mask {
        1 | 4 | 5 => "│",
        2 | 8 | 10 => "─",
        3 => "╰",
        6 => "╭",
        9 => "╯",
        12 => "╮",
        7 => "├",
        11 => "┴",
        13 => "┤",
        14 => "┬",
        15 => "┼",
        _ => " ",
    }
}

fn directions(symbol: &str) -> u8 {
    match symbol {
        "│" => N | S,
        "─" => E | W,
        "╰" => N | E,
        "╭" => E | S,
        "╯" => N | W,
        "╮" => S | W,
        "├" => N | E | S,
        "┴" => N | E | W,
        "┤" => N | S | W,
        "┬" => E | S | W,
        "┼" => N | E | S | W,
        _ => 0,
    }
}

fn rasterize(points: &[Point], area: Rect) -> Cells {
    let mut cells = Cells::new();
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a == b {
            continue;
        } // Duplicate waypoints must not create fake branches.
        if a.1 == b.1 && a.1 >= area.y as f64 && a.1 < area.bottom() as f64 {
            let (lo, hi) = (a.0.min(b.0), a.0.max(b.0));
            // Iterate only the visible row/column, even for huge off-screen paths.
            for x in area.x..area.right() {
                let p = x as f64;
                if p >= lo && p <= hi {
                    let mask = if p > lo { W } else { 0 } | if p < hi { E } else { 0 };
                    *cells.entry((x, a.1 as u16)).or_default() |= mask;
                }
            }
        } else if a.0 == b.0 && a.0 >= area.x as f64 && a.0 < area.right() as f64 {
            let (lo, hi) = (a.1.min(b.1), a.1.max(b.1));
            for y in area.y..area.bottom() {
                let p = y as f64;
                if p >= lo && p <= hi {
                    let mask = if p > lo { N } else { 0 } | if p < hi { S } else { 0 };
                    *cells.entry((a.0 as u16, y)).or_default() |= mask;
                }
            }
        }
    }
    cells
}

pub(super) fn path(f: &mut Frame, points: &[Point], color: Color) {
    for (position, mask) in rasterize(points, f.area()) {
        let cell = &mut f.buffer_mut()[position];
        let merged = mask | directions(cell.symbol());
        cell.set_symbol(symbol(merged))
            .set_style(Style::default().fg(color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn four_rounded_corners_and_straight_duplicate_points() {
        let points = [(2.0, 2.0), (6.0, 2.0), (6.0, 6.0), (2.0, 6.0), (2.0, 2.0)];
        let cells = rasterize(&points, Rect::new(0, 0, 10, 10));
        for (position, expected) in [((2, 2), "╭"), ((6, 2), "╮"), ((6, 6), "╯"), ((2, 6), "╰")]
        {
            assert_eq!(symbol(cells[&position]), expected);
        }
        let cells = rasterize(
            &[(1.0, 2.0), (4.0, 2.0), (4.0, 2.0), (7.0, 2.0)],
            Rect::new(0, 0, 10, 10),
        );
        assert_eq!(symbol(cells[&(4, 2)]), "─");
    }

    #[test]
    fn real_crossing_and_clipping() {
        let mut terminal = Terminal::new(TestBackend::new(10, 10)).unwrap();
        terminal
            .draw(|f| {
                path(f, &[(-100000.0, 4.0), (100000.0, 4.0)], Color::Blue);
                path(f, &[(4.0, -100000.0), (4.0, 100000.0)], Color::Red);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(4, 4)].symbol(), "┼");
        assert_eq!(buffer[(0, 4)].symbol(), "─");
        assert_eq!(buffer[(4, 0)].symbol(), "│");
        assert_eq!(buffer[(4, 4)].fg, Color::Red);
    }

    #[test]
    fn overlapping_segments_do_not_create_turns() {
        let cells = rasterize(
            &[(1.0, 2.0), (8.0, 2.0), (3.0, 2.0)],
            Rect::new(0, 0, 10, 5),
        );
        assert!(cells.values().all(|mask| symbol(*mask) == "─"));
    }
}
