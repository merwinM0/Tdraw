use super::*;

#[test]
fn waypoints_preview_save_select_and_delete() {
    for arrow in [false, true] {
        let mut a = connected(arrow);
        a.rectangles[0].connections.clear();
        a.save();
        let trigger = KeyCode::Char(if arrow { 'k' } else { 'l' });
        a.cursor = (10.0, 10.0);
        key(&mut a, trigger);
        a.cursor = (18.0, 16.0);
        key(&mut a, trigger);
        key(&mut a, trigger);
        a.cursor = (25.0, 16.0);
        key(&mut a, trigger);
        assert!(matches!(&a.mode, Mode::Connecting { waypoints, .. } if waypoints.len() == 2));
        assert!(
            App::load(a.path.clone(), a.size).unwrap().rectangles[0]
                .connections
                .is_empty()
        );
        a.cursor = (28.0, 12.0);
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &a)).unwrap();
        assert_eq!(terminal.backend().buffer()[(18, 16)].symbol(), "◆");
        assert_eq!(terminal.backend().buffer()[(22, 16)].symbol(), "─");
        a.cursor = (29.0, 10.0);
        key(&mut a, trigger);
        let c = &a.rectangles[0].connections[0];
        assert_eq!(c.waypoints, vec![(18.0, 16.0), (25.0, 16.0)]);
        assert_eq!(
            App::load(a.path.clone(), a.size).unwrap().rectangles,
            a.rectangles
        );
        terminal.draw(|f| ui(f, &a)).unwrap();
        assert_eq!(terminal.backend().buffer()[(18, 10)].symbol(), "╮");
        assert_eq!(terminal.backend().buffer()[(18, 16)].symbol(), "╰");
        // Moving an endpoint preserves absolute waypoint positions.
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Char('d'));
        key(&mut a, KeyCode::Enter);
        assert_eq!(
            a.rectangles[0].connections[0].waypoints,
            vec![(18.0, 16.0), (25.0, 16.0)]
        );
        a.cursor = (22.0, 16.0);
        assert_eq!(a.hovered_connection(), Some((0, 0)));
        key(&mut a, KeyCode::Enter);
        key(&mut a, KeyCode::Delete);
        assert!(a.rectangles[0].connections.is_empty());
        fs::remove_file(a.path).unwrap();
    }
}
#[test]
fn waypoints_cancel_enter_and_legacy_compatibility() {
    let mut a = connected(false);
    let before = a.rectangles.clone();
    a.cursor = (10.0, 10.0);
    key(&mut a, KeyCode::Char('l'));
    a.cursor = (20.0, 16.0);
    key(&mut a, KeyCode::Enter);
    assert!(matches!(&a.mode, Mode::Connecting { waypoints, .. } if waypoints.is_empty()));
    key(&mut a, KeyCode::Char('k')); // Wrong tool key does not add a point.
    assert!(matches!(&a.mode, Mode::Connecting { waypoints, .. } if waypoints.is_empty()));
    key(&mut a, KeyCode::Char('l'));
    key(&mut a, KeyCode::Char('q'));
    assert_eq!(a.rectangles, before);
    a.cursor = (10.0, 10.0);
    key(&mut a, KeyCode::Char('k'));
    a.cursor = (20.0, 16.0);
    key(&mut a, KeyCode::Char('k'));
    ctrl(&mut a, 'q');
    assert!(a.quit);
    assert_eq!(a.rectangles, before);
    let old: Connection = serde_json::from_str(r#"{"target":1,"from":{"side":"right","offset":0},"to":{"side":"left","offset":0},"arrow":false}"#).unwrap();
    assert!(old.waypoints.is_empty());
    a.rectangles[0].connections[0].waypoints = vec![(-1.0, 2.0)];
    a.save();
    assert!(App::load(a.path.clone(), a.size).is_err());
    fs::remove_file(a.path).unwrap();
}
