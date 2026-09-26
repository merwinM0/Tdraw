use super::*;

#[test]
fn drawing_preview_and_persistence() {
    let mut a = app();
    draw(&mut a);
    assert_eq!(
        a.rectangles[0],
        Shape {
            shape: ShapeKind::Rectangle,
            x: 39.0,
            y: 11.0,
            width: 2.0,
            height: 2.0,
            z: 0.0,
            text: String::new(),
            connections: Vec::new()
        }
    );
    assert_eq!(
        App::load(a.path.clone(), a.size).unwrap().rectangles,
        a.rectangles
    );
    fs::remove_file(a.path).unwrap();
}
#[test]
fn move_cancel_menu_and_delete() {
    let mut a = app();
    draw(&mut a);
    let original = a.rectangles.clone();
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('d'));
    key(&mut a, KeyCode::Char('m'));
    let cursor = a.cursor;
    key(&mut a, KeyCode::Char('d'));
    key(&mut a, KeyCode::Char('w'));
    assert_eq!(a.cursor, cursor);
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('q'));
    assert_eq!(a.rectangles, original);
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Backspace);
    assert!(
        App::load(a.path.clone(), a.size)
            .unwrap()
            .rectangles
            .is_empty()
    );
    fs::remove_file(a.path).unwrap();
}
#[test]
fn layers_and_bounds() {
    let mut a = app();
    draw(&mut a);
    a.cursor = (40.0, 12.0);
    draw(&mut a);
    assert_eq!(a.hovered(), Some(1));
    a.cursor = (41.0, 12.0);
    assert_eq!(a.hovered(), None);
    a.resize(1, 1);
    key(&mut a, KeyCode::Char('a'));
    assert_eq!(a.cursor, (0.0, 0.0));
    fs::remove_file(a.path).unwrap();
}
#[test]
fn quit_cancels_unconfirmed_changes() {
    let mut a = app();
    draw(&mut a);
    let original = a.rectangles.clone();
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('d'));
    key(&mut a, KeyCode::Char('m'));
    ctrl(&mut a, 'q');
    assert!(a.quit);
    assert_eq!(a.rectangles, original);
    fs::remove_file(a.path).unwrap();
    let mut a = app();
    ctrl(&mut a, 'd');
    ctrl(&mut a, 'q');
    assert!(a.quit);
    assert!(!a.path.exists());
}
#[test]
fn render_small_and_clipped_canvases() {
    let mut a = app();
    a.rectangles.push(Shape {
        shape: ShapeKind::Rectangle,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        width: 100000.0,
        height: 100000.0,
        text: String::new(),
        connections: Vec::new(),
    });
    a.mode = Mode::Menu {
        index: 0,
        original: a.rectangles[0].clone(),
        cursor: a.cursor,
        item: 0,
    };
    for (w, h) in [(1, 1), (8, 4), (80, 24)] {
        a.resize(w, h);
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| ui(f, &a)).unwrap();
    }
}
#[test]
fn text_editing_grows_saves_and_loads() {
    let mut a = app();
    draw(&mut a);
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('r'));
    for c in "中文qwasd".chars() {
        key(&mut a, KeyCode::Char(c));
    }
    ctrl(&mut a, 'j');
    key(&mut a, KeyCode::Char('好'));
    assert_eq!(a.rectangles[0].width, 11.0);
    assert_eq!(a.rectangles[0].height, 4.0);
    key(&mut a, KeyCode::Enter);
    let loaded = App::load(a.path.clone(), a.size).unwrap();
    assert_eq!(loaded.rectangles[0].text, "中文qwasd\n好");
    assert_eq!(loaded.rectangles, a.rectangles);
    fs::remove_file(a.path).unwrap();
}
#[test]
fn text_cancel_backspace_and_quit() {
    let mut a = app();
    draw(&mut a);
    let original = a.rectangles.clone();
    key(&mut a, KeyCode::Enter);
    key(&mut a, KeyCode::Char('r'));
    for c in "e\u{301}".chars() {
        key(&mut a, KeyCode::Char(c));
    }
    key(&mut a, KeyCode::Backspace);
    assert!(a.rectangles[0].text.is_empty());
    key(&mut a, KeyCode::Esc);
    assert_eq!(a.rectangles, original);
    assert!(matches!(a.mode, Mode::Moving { .. }));
    key(&mut a, KeyCode::Char('r'));
    key(&mut a, KeyCode::Char('中'));
    ctrl(&mut a, 'q');
    assert!(a.quit);
    assert_eq!(a.rectangles, original);
    fs::remove_file(a.path).unwrap();
}
#[test]
fn legacy_json_defaults_to_empty_text() {
    let r: Shape = serde_json::from_str(r#"{"x":0,"y":0,"z":0,"width":3,"height":3}"#).unwrap();
    assert!(r.text.is_empty());
}
#[test]
fn invalid_connection_is_rejected_without_overwrite() {
    let a = app();
    let data = r#"[{"x":0,"y":0,"z":0,"width":2,"height":2,"connections":[{"target":9,"from":{"side":"left","offset":0},"to":{"side":"right","offset":0},"arrow":true}]}]"#;
    fs::write(&a.path, data).unwrap();
    assert!(App::load(a.path.clone(), a.size).is_err());
    assert_eq!(fs::read_to_string(&a.path).unwrap(), data);
    fs::remove_file(a.path).unwrap();
}
#[test]
fn invalid_file_is_preserved() {
    let a = app();
    fs::write(&a.path, "not json").unwrap();
    assert!(App::load(a.path.clone(), a.size).is_err());
    assert_eq!(fs::read_to_string(&a.path).unwrap(), "not json");
    fs::remove_file(a.path).unwrap();
}
