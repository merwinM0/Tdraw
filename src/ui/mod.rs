pub(crate) mod theme;
use crate::{
    app::{App, Mode},
    connection,
    model::Rectangle,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use theme::{INK, ITEMS, PURPLE, SOFT};
use unicode_width::UnicodeWidthStr;

// Draw only visible cells: clipping must not manufacture borders at viewport edges.
fn draw_rectangle(f: &mut Frame, r: &Rectangle, color: Color) {
    let area = f.area();
    let left = r.x.floor();
    let top = r.y.floor();
    let right = left + r.width.floor() - 1.0;
    let bottom = top + r.height.floor() - 1.0;
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let px = x as f64;
            let py = y as f64;
            if px < left || px > right || py < top || py > bottom {
                continue;
            }
            let symbol = if py == top && px == left {
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

fn draw_text(f: &mut Frame, r: &Rectangle) {
    let area = f.area();
    let left = r.x.floor() + 1.0;
    let top = r.y.floor() + 1.0;
    if left >= area.right() as f64 || top >= area.bottom() as f64 {
        return;
    }
    let width = (r.width.floor() - 2.0)
        .max(0.0)
        .min(area.right() as f64 - left) as u16;
    let height = (r.height.floor() - 2.0)
        .max(0.0)
        .min(area.bottom() as f64 - top) as u16;
    f.render_widget(
        Paragraph::new(r.text.as_str()).style(Style::default().fg(INK).bg(Color::White)),
        Rect::new(left as u16, top as u16, width, height),
    );
}

fn panel() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(205, 198, 225)))
        .style(Style::default().fg(INK).bg(Color::White))
}

pub(crate) fn ui(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Color::White).fg(INK)),
        area,
    );
    let hovered_connection = if matches!(app.mode, Mode::Normal) {
        app.hovered_connection()
    } else {
        None
    };
    for (source, r) in app.rectangles.iter().enumerate() {
        for (index, c) in r.connections.iter().enumerate() {
            let color = if matches!(app.mode, Mode::SelectedConnection { source: s, index: i } if s == source && i == index)
            {
                PURPLE
            } else if hovered_connection == Some((source, index)) {
                Color::Blue
            } else {
                INK
            };
            connection::draw(
                f,
                (r, c.from),
                c.to.point(&app.rectangles[c.target]),
                Some(c.to),
                c.arrow,
                color,
                &c.waypoints,
            );
        }
    }
    if let Mode::Connecting {
        source,
        anchor,
        arrow,
        ref waypoints,
    } = app.mode
    {
        let target = app.boundary().filter(|(i, _)| *i != source);
        connection::draw(
            f,
            (&app.rectangles[source], anchor),
            app.cursor,
            target.map(|(_, a)| a),
            arrow,
            PURPLE,
            waypoints,
        );
        for &(x, y) in waypoints {
            if x < area.width as f64 && y < area.height as f64 {
                f.render_widget(
                    Paragraph::new("◆").style(Style::default().fg(PURPLE)),
                    Rect::new(x as u16, y as u16, 1, 1),
                );
            }
        }
    }
    let selected = match app.mode {
        Mode::Connecting { source, .. } => Some(source),
        Mode::Moving { index, .. } | Mode::Menu { index, .. } | Mode::Editing { index, .. } => {
            Some(index)
        }
        _ => None,
    };
    let hovered = if matches!(app.mode, Mode::Normal | Mode::Connecting { .. }) {
        app.hovered()
    } else {
        None
    };
    let mut order: Vec<_> = app.rectangles.iter().enumerate().collect();
    order.sort_by(|(ai, a), (bi, b)| a.z.total_cmp(&b.z).then(ai.cmp(bi)));
    for (i, r) in order {
        draw_rectangle(
            f,
            r,
            if selected == Some(i) {
                PURPLE
            } else if hovered == Some(i) {
                Color::Blue
            } else {
                INK
            },
        );
        draw_text(f, r);
    }
    if let Mode::Drawing { anchor } = app.mode {
        draw_rectangle(f, &app.preview(anchor), PURPLE);
    }
    let (mode, help) = match app.mode {
        Mode::Normal => (
            "选择",
            "WASD 移动 · Ctrl+D 矩形 · L 连线 · K 箭头 · Enter 选择 · Ctrl+Q 退出",
        ),
        Mode::SelectedConnection { .. } => (
            "连接已选中",
            "Backspace/Delete 删除连接 · Q/Esc/Enter 取消选择 · WASD 移动光标",
        ),
        Mode::Connecting { arrow, .. } => (
            if arrow { "箭头" } else { "连线" },
            if arrow {
                "WASD 移动 · K 中间点/终点 · Enter 终点 · Q 取消"
            } else {
                "WASD 移动 · L 中间点/终点 · Enter 终点 · Q 取消"
            },
        ),
        Mode::Drawing { .. } => (
            "矩形",
            "WASD 调整对角点 · Enter 保存 · Q 取消 · Ctrl+Q 退出",
        ),
        Mode::Moving { .. } => (
            "移动",
            "WASD 移动 · R 文字 · Enter 保存 · Q 还原 · ⌫ 删除 · M 菜单",
        ),
        Mode::Editing { .. } => (
            "文字",
            "输入文字 · Ctrl+J 换行 · ⌫ 退格 · Enter 保存 · Esc 取消",
        ),
        Mode::Menu { .. } => ("菜单", "W/S 选择 · Enter 预览 · Q 返回 · Ctrl+Q 退出"),
    };
    if area.width >= 20 && area.height >= 8 {
        let width = area.width.saturating_sub(4).min(66);
        f.render_widget(
            Paragraph::new(format!(" Tdraw   │   ↖ 选择    ▣ 矩形 Ctrl+D   │   {mode}"))
                .block(panel())
                .style(Style::default().fg(PURPLE).bg(SOFT)),
            Rect::new((area.width - width) / 2, 0, width, 3),
        );
        f.render_widget(
            Paragraph::new(format!(
                " {help}\n {}  │  {} 区块  │  {},{}{}",
                app.message,
                app.rectangles.len(),
                app.cursor.0,
                app.cursor.1,
                if app.dirty { " · 未保存" } else { "" }
            ))
            .style(Style::default().fg(INK).bg(SOFT)),
            Rect::new(0, area.height - 2, area.width, 2),
        );
    }
    let (x, y) = (app.cursor.0 as u16, app.cursor.1 as u16);
    if let Mode::Editing { index, .. } = app.mode {
        let r = &app.rectangles[index];
        let tx = r.x.floor() + 1.0 + r.text.split('\n').next_back().unwrap_or("").width() as f64;
        let ty = r.y.floor() + r.text.split('\n').count() as f64;
        if tx < area.width as f64 && ty < area.height as f64 {
            f.set_cursor_position((tx as u16, ty as u16));
        }
    } else if x < area.width && y < area.height {
        f.render_widget(
            Paragraph::new("●").style(Style::default().fg(Color::Black).bg(Color::White)),
            Rect::new(x, y, 1, 1),
        );
    }
    if let Mode::Menu { item, .. } = app.mode {
        let width = area.width.min(26);
        let height = area.height.min(7);
        let mx = if x.saturating_add(2).saturating_add(width) <= area.width {
            x + 2
        } else {
            x.saturating_sub(width)
        };
        let my = y.min(area.height.saturating_sub(height));
        let popup = Rect::new(mx, my, width, height);
        f.render_widget(Clear, popup);
        f.render_widget(panel().title(" 区块操作 · 预览 "), popup);
        for (i, label) in ITEMS.iter().enumerate() {
            if i as u16 + 2 >= height {
                break;
            }
            f.render_widget(
                Paragraph::new(format!(" {} {label}", if i == item { "›" } else { " " })).style(
                    Style::default()
                        .fg(if i == item { PURPLE } else { INK })
                        .bg(if i == item { SOFT } else { Color::White }),
                ),
                Rect::new(mx + 1, my + 1 + i as u16, width.saturating_sub(2), 1),
            );
        }
    }
}
