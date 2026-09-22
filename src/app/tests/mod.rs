mod connections;
mod core;
mod waypoints;

use crate::{
    connection::Connection,
    ui::{theme::PURPLE, ui},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, style::Color};

use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn app() -> App {
    let path = std::env::temp_dir().join(format!(
        "tdraw-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    App::load(path, (80, 24)).unwrap()
}
fn key(app: &mut App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}
fn ctrl(app: &mut App, c: char) {
    app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}
fn draw(app: &mut App) {
    ctrl(app, 'd');
    key(app, KeyCode::Char('a'));
    key(app, KeyCode::Char('w'));
    key(app, KeyCode::Enter);
}
fn connected(arrow: bool) -> App {
    let mut a = app();
    a.cursor = (10.0, 10.0);
    draw(&mut a);
    a.cursor = (30.0, 10.0);
    draw(&mut a);
    a.cursor = (10.0, 10.0);
    key(&mut a, KeyCode::Char(if arrow { 'k' } else { 'l' }));
    let original = a.rectangles.clone();
    key(&mut a, KeyCode::Char('d'));
    assert_eq!(a.rectangles, original);
    a.cursor = (29.0, 10.0);
    key(&mut a, KeyCode::Enter);
    assert!(matches!(a.mode, Mode::Normal));
    a
}
