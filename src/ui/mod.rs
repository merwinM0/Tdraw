mod shapes;
use shapes::{draw_shape, draw_text};
pub(crate) mod theme;
use crate::{
    app::{App, Mode},
    connection,
    model::ShapeKind,
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use theme::{INK, ITEMS, PURPLE, SOFT};
use unicode_width::UnicodeWidthStr;

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
            let target = &app.rectangles[c.target];
            let (from, to) = c.anchors(r, target);
            connection::draw(
                f,
                (r, from),
                to.point(target),
                Some(to),
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
        let mut from = anchor;
        let mut to = target.map(|(_, a)| a);
        let mut endpoint = app.cursor;
        if arrow {
            from = connection::Anchor::facing(
                &app.rectangles[source],
                waypoints.first().copied().unwrap_or(app.cursor),
            );
            if let Some((index, _)) = target {
                let c = connection::Connection {
                    target: index,
                    from: anchor,
                    to: to.unwrap(),
                    arrow,
                    waypoints: waypoints.clone(),
                };
                let anchors = c.anchors(&app.rectangles[source], &app.rectangles[index]);
                from = anchors.0;
                to = Some(anchors.1);
                endpoint = anchors.1.point(&app.rectangles[index]);
            }
        }
        connection::draw(
            f,
            (&app.rectangles[source], from),
            endpoint,
            to,
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
        draw_shape(
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
    if let Mode::Drawing { anchor, .. } = app.mode {
        draw_shape(f, &app.preview(anchor), PURPLE);
    }
    let (mode, help) = match app.mode {
        Mode::Normal => (
            "选择",
            "Ctrl+D 矩形 · Ctrl+W 圆形 · Ctrl+A 菱形 · L 连线 · K 箭头 · Enter 选择",
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
        Mode::Drawing { shape, .. } => (
            match shape {
                ShapeKind::Rectangle => "矩形",
                ShapeKind::Ellipse => "圆形",
                ShapeKind::Diamond => "菱形",
            },
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
            Paragraph::new(" Tdraw | 终端图表绘制工具")
                .block(panel())
                .style(Style::default().fg(PURPLE).bg(SOFT)),
            Rect::new((area.width - width) / 2, 0, width, 3),
        );
        f.render_widget(
            Paragraph::new(format!(
                " {help}\n {mode} · {}  │  {} 方块  │  {},{}{}",
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
        let (left, top, _, _) = r.text_region();
        let tx = left + r.text.split('\n').next_back().unwrap_or("").width() as f64;
        let ty = top + r.text.split('\n').count() as f64 - 1.0;
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
