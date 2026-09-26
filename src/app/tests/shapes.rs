use super::*;
use crate::connection::Side;
use unicode_width::UnicodeWidthStr;

fn shape_app(kind: ShapeKind) -> App {
    let mut a = app();
    a.cursor = (5.0, 5.0);
    ctrl(
        &mut a,
        match kind {
            ShapeKind::Rectangle => 'd',
            ShapeKind::Ellipse => 'w',
            ShapeKind::Diamond => 'a',
        },
    );
    a.cursor = (25.0, 15.0);
    key(&mut a, KeyCode::Enter);
    a
}

#[test]
fn draw_edit_save_and_delete_new_shapes() {
    for kind in [ShapeKind::Ellipse, ShapeKind::Diamond] {
        let mut a = shape_app(kind);
        assert_eq!(a.rectangles[0].shape, kind);
        assert!(!a.rectangles[0].contains(5.0, 5.0));
        assert!(a.rectangles[0].on_border(15.0, 5.0));
        a.cursor = (15.0, 10.0);
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('r'));
        for c in "中文很长的文字hello".chars() {
            key(&mut a, KeyCode::Char(c));
        }
        ctrl(&mut a, 'j');
        key(&mut a, KeyCode::Char('好'));
        let r = &a.rectangles[0];
        let (x, y, w, h) = r.text_region();
        for (row, line) in r.text.split('\n').enumerate() {
            assert!(line.width() as f64 <= w);
            assert!((row as f64) < h);
            for col in 0..line.width() {
                assert!(r.contains(x + col as f64, y + row as f64));
                assert!(!r.on_border(x + col as f64, y + row as f64));
            }
        }
        key(&mut a, KeyCode::Enter);
        assert_eq!(
            App::load(a.path.clone(), a.size).unwrap().rectangles,
            a.rectangles
        );
        a.cursor = a.rectangles[0].center();
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Backspace);
        assert!(a.rectangles.is_empty());
        fs::remove_file(a.path).unwrap();
    }
}

#[test]
fn shape_text_padding_and_anchors_across_sizes() {
    let mut a = shape_app(ShapeKind::Diamond);
    for kind in [ShapeKind::Ellipse, ShapeKind::Diamond] {
        for width in 1..25 {
            for height in 1..20 {
                let r = &mut a.rectangles[0];
                r.shape = kind;
                r.width = width as f64;
                r.height = height as f64;
                for side in [Side::Left, Side::Right, Side::Top, Side::Bottom] {
                    for offset in [0.0, 0.25, 0.5, 0.75, 1.0] {
                        let anchor = Anchor { side, offset };
                        let p = anchor.point(r);
                        assert!(
                            r.on_border(p.0, p.1),
                            "{kind:?} {width}x{height} {anchor:?} {p:?}"
                        );
                    }
                }
                r.text = "中文\nabc".into();
                r.fit_text();
                let (x, y, _, _) = r.text_region();
                for (row, text) in r.text.split('\n').enumerate() {
                    for col in 0..text.width() {
                        assert!(!r.on_border(x + col as f64, y + row as f64));
                        assert!(r.contains(x + col as f64, y + row as f64));
                    }
                }
            }
        }
    }
    fs::remove_file(a.path).unwrap();
}

#[test]
fn arrows_choose_facing_sides_and_render_current_tip() {
    let mut a = connected(true);
    a.rectangles[0].x = 30.0;
    a.rectangles[0].y = 10.0;
    a.rectangles[0].width = 11.0;
    a.rectangles[0].height = 7.0;
    a.rectangles[1].width = 11.0;
    a.rectangles[1].height = 7.0;
    for kind in [ShapeKind::Rectangle, ShapeKind::Ellipse, ShapeKind::Diamond] {
        a.rectangles[1].shape = kind;
        for (x, y, side, glyph) in [
            (55.0, 10.0, Side::Left, "▶"),
            (5.0, 10.0, Side::Right, "◀"),
            (30.0, 25.0, Side::Top, "▼"),
            (30.0, 0.0, Side::Bottom, "▲"),
        ] {
            a.rectangles[1].x = x;
            a.rectangles[1].y = y;
            let r = &a.rectangles[0];
            let target = &a.rectangles[1];
            let c = &r.connections[0];
            let (_, to) = c.anchors(r, target);
            assert_eq!(to.side, side);
            let p = to.point(target);
            let tip = match side {
                Side::Left => (p.0 - 1.0, p.1),
                Side::Right => (p.0 + 1.0, p.1),
                Side::Top => (p.0, p.1 - 1.0),
                Side::Bottom => (p.0, p.1 + 1.0),
            };
            assert!(crate::connection::contains(r, target, c, tip));
            a.cursor = (0.0, 20.0);
            let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 40)).unwrap();
            terminal.draw(|f| ui(f, &a)).unwrap();
            assert_eq!(
                terminal.backend().buffer()[(tip.0 as u16, tip.1 as u16)].symbol(),
                glyph
            );
        }
    }
    fs::remove_file(a.path).unwrap();
}

#[test]
fn mixed_shapes_connect_and_render_transparent_corners() {
    let mut a = shape_app(ShapeKind::Ellipse);
    a.cursor = (40.0, 5.0);
    ctrl(&mut a, 'a');
    a.cursor = (60.0, 15.0);
    key(&mut a, KeyCode::Enter);
    a.cursor = (25.0, 10.0);
    key(&mut a, KeyCode::Char('k'));
    assert!(matches!(a.mode, Mode::Connecting { .. }));
    a.cursor = (40.0, 10.0);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.rectangles[0].connections[0].target, 1);
    assert_eq!(
        App::load(a.path.clone(), a.size).unwrap().rectangles,
        a.rectangles
    );
    a.cursor = (0.0, 20.0);
    let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();
    terminal.draw(|f| ui(f, &a)).unwrap();
    assert_eq!(terminal.backend().buffer()[(5, 5)].symbol(), " ");
    assert_eq!(terminal.backend().buffer()[(40, 5)].symbol(), " ");
    assert_ne!(terminal.backend().buffer()[(15, 5)].symbol(), " ");
    assert_ne!(terminal.backend().buffer()[(50, 5)].symbol(), " ");
    let title: String = (0..80)
        .map(|x| terminal.backend().buffer()[(x, 1)].symbol())
        .collect();
    // Wide CJK cells include padding in the terminal buffer; the ASCII prefix is exact.
    assert!(title.contains("Tdraw |"));
    assert!(title.contains('终'));
    a.cursor = (50.0, 10.0);
    key(&mut a, KeyCode::Enter);
    let before = a.rectangles.clone();
    for _ in 0..8 {
        key(&mut a, KeyCode::Char('s'));
    }
    key(&mut a, KeyCode::Char('q'));
    assert_eq!(a.rectangles, before);
    fs::remove_file(a.path).unwrap();
}

#[test]
fn shape_cancel_and_legacy_rectangle() {
    let mut a = app();
    ctrl(&mut a, 'w');
    key(&mut a, KeyCode::Char('q'));
    assert!(a.rectangles.is_empty());
    ctrl(&mut a, 'a');
    ctrl(&mut a, 'q');
    assert!(a.quit);
    let r: Shape = serde_json::from_str(r#"{"x":0,"y":0,"z":0,"width":3,"height":3}"#).unwrap();
    assert_eq!(r.shape, ShapeKind::Rectangle);
}
