use super::*;

#[test]
fn connections_persist_follow_and_delete() {
    for arrow in [false, true] {
        let mut a = connected(arrow);
        let c = a.rectangles[0].connections[0].clone();
        assert_eq!(c.target, 1);
        assert_eq!(c.arrow, arrow);
        let before = c.to.point(&a.rectangles[1]);
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('d'));
        assert_eq!(c.to.point(&a.rectangles[1]), (before.0 + 1.0, before.1));
        key(&mut a, KeyCode::Char('q'));
        assert_eq!(c.to.point(&a.rectangles[1]), before);
        assert_eq!(
            App::load(a.path.clone(), a.size).unwrap().rectangles,
            a.rectangles
        );
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &a)).unwrap();
        assert_eq!(terminal.backend().buffer()[(20, 10)].symbol(), "─");
        if arrow {
            assert_eq!(terminal.backend().buffer()[(28, 10)].symbol(), "▶");
        }
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Backspace);
        assert!(a.rectangles[0].connections.is_empty());
        fs::remove_file(a.path).unwrap();
    }
}
#[test]
fn connection_requires_distinct_boundaries_and_cancels() {
    let mut a = app();
    a.cursor = (10.0, 10.0);
    ctrl(&mut a, 'd');
    a.cursor = (15.0, 15.0);
    key(&mut a, KeyCode::Enter);
    a.cursor = (12.0, 12.0);
    key(&mut a, KeyCode::Char('l'));
    assert!(matches!(a.mode, Mode::Normal));
    a.cursor = (10.0, 12.0);
    key(&mut a, KeyCode::Char('l'));
    key(&mut a, KeyCode::Enter);
    assert!(matches!(a.mode, Mode::Connecting { .. }));
    key(&mut a, KeyCode::Char('q'));
    assert!(a.rectangles[0].connections.is_empty());
    key(&mut a, KeyCode::Char('k'));
    ctrl(&mut a, 'q');
    assert!(a.quit);
    fs::remove_file(a.path).unwrap();
}
#[test]
fn connection_indices_remap_and_anchors_resize() {
    let mut a = connected(true);
    a.cursor = (50.0, 10.0);
    draw(&mut a);
    let mut c = a.rectangles[0].connections[0].clone();
    c.target = 2;
    a.rectangles[0].connections.push(c);
    a.cursor = (29.0, 10.0);
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Backspace);
    assert_eq!(a.rectangles[0].connections.len(), 1);
    assert_eq!(a.rectangles[0].connections[0].target, 1);
    let anchor = Anchor::at(&a.rectangles[0], (10.0, 10.0)).unwrap();
    a.rectangles[0].width += 10.0;
    assert_eq!(anchor.point(&a.rectangles[0]), (20.0, 10.0));
    fs::remove_file(a.path).unwrap();
}
#[test]
fn select_delete_connections_preserves_boxes_and_other_links() {
    for arrow in [false, true] {
        for delete in [KeyCode::Backspace, KeyCode::Delete] {
            let mut a = connected(arrow);
            let mut other = a.rectangles[0].connections[0].clone();
            other.arrow = !arrow;
            a.rectangles[0].connections.insert(0, other.clone());
            let before = a.rectangles.clone();
            a.cursor = (20.0, 10.0);
            assert_eq!(a.hovered_connection(), Some((0, 1)));
            key(&mut a, KeyCode::Enter);
            assert!(matches!(
                a.mode,
                Mode::SelectedConnection {
                    source: 0,
                    index: 1
                }
            ));
            key(&mut a, KeyCode::Char('s'));
            assert_eq!(a.rectangles, before);
            key(&mut a, delete);
            let mut expected = before;
            expected[0].connections.pop();
            assert_eq!(a.rectangles, expected);
            assert!(matches!(a.mode, Mode::Normal));
            assert_eq!(
                App::load(a.path.clone(), a.size).unwrap().rectangles,
                expected
            );
            fs::remove_file(a.path).unwrap();
        }
    }
}
#[test]
fn connection_hover_selection_cancel_and_arrow_tip() {
    let mut a = connected(true);
    let before = a.rectangles.clone();
    a.cursor = (28.0, 10.0); // Arrowhead is selectable too.
    assert_eq!(a.hovered_connection(), Some((0, 0)));
    let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui(f, &a)).unwrap();
    assert_eq!(terminal.backend().buffer()[(20, 10)].fg, Color::Blue);
    for cancel in [KeyCode::Char('q'), KeyCode::Esc, KeyCode::Enter] {
        key(&mut a, KeyCode::Enter);
        terminal.draw(|f| ui(f, &a)).unwrap();
        assert_eq!(terminal.backend().buffer()[(20, 10)].fg, PURPLE);
        key(&mut a, cancel);
        assert!(matches!(a.mode, Mode::Normal));
        assert_eq!(a.rectangles, before);
    }
    key(&mut a, KeyCode::Enter);
    ctrl(&mut a, 'q');
    assert!(a.quit);
    assert_eq!(a.rectangles, before);
    fs::remove_file(a.path).unwrap();
}
#[test]
fn connection_hit_testing_respects_bends_occlusion_and_moves() {
    let mut a = connected(false);
    a.rectangles[1].y += 5.0;
    a.cursor = (28.0, 12.0);
    assert_eq!(a.hovered_connection(), Some((0, 0)));
    a.cursor = (28.0, 10.0);
    assert_eq!(a.hovered_connection(), Some((0, 0)));
    a.cursor = (20.0, 11.0);
    assert_eq!(a.hovered_connection(), None);
    a.cursor = (10.0, 10.0);
    assert_eq!(a.hovered_connection(), None);
    key(&mut a, KeyCode::Enter);
    assert!(matches!(a.mode, Mode::Moving { .. }));
    key(&mut a, KeyCode::Char('q'));
    let mut blocker = a.preview((19.0, 9.0));
    blocker.x = 19.0;
    blocker.y = 9.0;
    blocker.width = 4.0;
    blocker.height = 4.0;
    a.rectangles.push(blocker);
    a.cursor = (20.0, 10.0);
    assert_eq!(a.hovered_connection(), None);
    fs::remove_file(a.path).unwrap();
}
