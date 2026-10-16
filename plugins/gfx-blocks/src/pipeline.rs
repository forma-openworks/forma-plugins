//! ` ```pipeline ` — карта этапов работы над моделью или шотом: что готово, что идёт
//! сейчас, что впереди.
//!
//! ```text
//! Concept: done
//! Blockout: done
//! Modeling: now — фюзеляж и крылья
//! Retopology
//! UV: skip — авто-развёртка хватает
//! ```
//!
//! Статус — после двоеточия: `done` (готово), `now` (идёт), `next` (впереди, по
//! умолчанию), `skip` (пропущен); можно по-русски: «готово», «сейчас», «дальше»,
//! «пропуск». Заметка — после тире; её видно при наведении, а у текущего этапа — и под
//! картой.

use forma_plugin_api::ui::{self, Align, Element, Shape};

use crate::util::{Cx, fit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Done,
    Now,
    Next,
    Skip,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    pub name: String,
    pub status: Status,
    pub note: String,
}

fn status_of(word: &str) -> Option<Status> {
    match word.trim().to_lowercase().as_str() {
        "done" | "готово" | "готов" | "✓" | "x" => Some(Status::Done),
        "now" | "current" | "сейчас" | "идёт" | "идет" | "в работе" => Some(Status::Now),
        "next" | "todo" | "дальше" | "впереди" | "" => Some(Status::Next),
        "skip" | "пропуск" | "пропущен" => Some(Status::Skip),
        _ => None,
    }
}

pub fn parse(source: &str) -> Vec<Stage> {
    let mut out = Vec::new();
    for line in source.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("//")) {
        let line = line.trim_start_matches(['-', '*']).trim();
        let (head, note) = match line.split_once(" — ").or_else(|| line.split_once(" - ")) {
            Some((h, n)) => (h.trim(), n.trim().to_owned()),
            None => (line, String::new()),
        };
        let (name, status) = match head.rsplit_once(':') {
            Some((n, s)) if status_of(s).is_some() => (n.trim(), status_of(s).unwrap_or(Status::Next)),
            _ => (head, Status::Next),
        };
        out.push(Stage { name: name.to_owned(), status, note });
    }
    out
}

/// Ширина карты в единицах холста (программа вписывает её в колонку заметки).
const WIDTH: f32 = 640.0;
const PILL_W: f32 = 118.0;
const PILL_H: f32 = 40.0;
const GAP_X: f32 = 34.0;
const ROW_GAP: f32 = 30.0;
const PAD: f32 = 12.0;

pub fn render(source: &str, cx: &Cx) -> Result<Element, String> {
    let stages = parse(source);
    if stages.is_empty() {
        return Err(cx.tr("пустой pipeline: по этапу в строке, «Modeling: now»", "empty pipeline: one stage per line, “Modeling: now”").into());
    }
    let per_row = (((WIDTH - 2.0 * PAD + GAP_X) / (PILL_W + GAP_X)).floor() as usize).max(1);
    let rows = stages.len().div_ceil(per_row);
    let height = 2.0 * PAD + rows as f32 * PILL_H + (rows - 1) as f32 * ROW_GAP;
    let pos = |i: usize| {
        let (r, c) = (i / per_row, i % per_row);
        (PAD + c as f32 * (PILL_W + GAP_X), PAD + r as f32 * (PILL_H + ROW_GAP))
    };
    let mut shapes = Vec::new();
    // Стрелки между этапами: в строке — вправо, при переносе — вниз и к началу строки.
    for i in 1..stages.len() {
        let (x0, y0) = pos(i - 1);
        let (x1, y1) = pos(i);
        let color = if stages[i - 1].status == Status::Done { "muted" } else { "faint" };
        if y0 == y1 {
            let (a, b) = ([x0 + PILL_W + 4.0, y0 + PILL_H / 2.0], [x1 - 6.0, y1 + PILL_H / 2.0]);
            shapes.push(ui::line(a, b, color, 1.5));
            shapes.push(arrow_head(b, [1.0, 0.0], color));
        } else {
            let mid = y0 + PILL_H + ROW_GAP / 2.0;
            let points = vec![[x0 + PILL_W / 2.0, y0 + PILL_H + 3.0], [x0 + PILL_W / 2.0, mid], [x1 + PILL_W / 2.0, mid], [x1 + PILL_W / 2.0, y1 - 6.0]];
            shapes.push(Shape::Line { points, color: color.into(), width: 1.5, dashed: false });
            shapes.push(arrow_head([x1 + PILL_W / 2.0, y1 - 6.0], [0.0, 1.0], color));
        }
    }
    for (i, s) in stages.iter().enumerate() {
        let (x, y) = pos(i);
        let (fill, stroke, text, width) = match s.status {
            Status::Done => ("#00000000", "success", "text", 1.2),
            Status::Now => ("accent", "accent", "#ffffff", 2.0),
            Status::Next => ("#00000000", "border", "muted", 1.0),
            Status::Skip => ("#00000000", "border", "faint", 1.0),
        };
        let tooltip = if s.note.is_empty() { String::new() } else { format!("{}: {}", s.name, s.note) };
        shapes.push(Shape::Rect { x, y, w: PILL_W, h: PILL_H, fill: Some(fill.into()), stroke: Some(stroke.into()), stroke_width: width, radius: vec![PILL_H / 2.0], tooltip });
        let mark = match s.status {
            Status::Done => "✓ ",
            Status::Skip => "– ",
            _ => "",
        };
        let size = 12.5;
        let label = fit(&format!("{mark}{}", s.name), PILL_W - 18.0, size);
        shapes.push(Shape::Text {
            x: x + PILL_W / 2.0,
            y: y + PILL_H / 2.0,
            text: label,
            size,
            color: if s.status == Status::Done { "success".into() } else { text.into() },
            align: Align::Center,
            valign: Align::Center,
            bold: s.status == Status::Now,
            mono: false,
        });
    }
    let done = stages.iter().filter(|s| s.status == Status::Done).count();
    let counted = stages.iter().filter(|s| s.status != Status::Skip).count();
    let mut summary = if cx.english { format!("{done} of {counted} stages done") } else { format!("Готово этапов: {done} из {counted}") };
    if let Some(now) = stages.iter().find(|s| s.status == Status::Now) {
        summary.push_str(&if cx.english { format!(" · now: {}", now.name) } else { format!(" · сейчас: {}", now.name) });
        if !now.note.is_empty() {
            summary.push_str(&format!(" — {}", now.note));
        }
    }
    Ok(ui::column([ui::canvas(WIDTH, height, shapes), ui::hint(summary)]))
}

/// Наконечник стрелки в точке `tip`, направление `dir` (единичное).
fn arrow_head(tip: [f32; 2], dir: [f32; 2], color: &str) -> Shape {
    let (l, w) = (7.0, 4.0);
    let back = [tip[0] - dir[0] * l, tip[1] - dir[1] * l];
    let side = [-dir[1] * w, dir[0] * w];
    Shape::Polygon { points: vec![tip, [back[0] + side[0], back[1] + side[1]], [back[0] - side[0], back[1] - side[1]]], fill: Some(color.into()), stroke: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_read_status_and_note() {
        let s = parse("Concept: done\n- Modeling: сейчас — фюзеляж и крылья\nRetopology\nUV: skip - авто\n\n// комментарий\nTime: 0:04");
        assert_eq!(s.len(), 5);
        assert_eq!((s[0].status, s[1].status, s[2].status, s[3].status), (Status::Done, Status::Now, Status::Next, Status::Skip));
        assert_eq!((s[1].name.as_str(), s[1].note.as_str()), ("Modeling", "фюзеляж и крылья"));
        assert_eq!(s[4].name, "Time: 0:04", "двоеточие без статуса — часть названия");
    }

    #[test]
    fn map_wraps_rows_and_sums_up() {
        let src = (1..=7).map(|i| format!("Stage {i}{}", if i < 3 { ": done" } else if i == 3 { ": now" } else { "" })).collect::<Vec<_>>().join("\n");
        let Element::Column { children, .. } = render(&src, &Cx::default()).unwrap() else { panic!() };
        let Element::Canvas { height, shapes, .. } = &children[0] else { panic!() };
        assert_eq!(*height, 2.0 * PAD + 2.0 * PILL_H + ROW_GAP, "7 этапов — две строки");
        assert_eq!(shapes.iter().filter(|s| matches!(s, Shape::Rect { .. })).count(), 7);
        let json = children[1].to_json();
        assert!(json.contains("Готово этапов: 2 из 7") && json.contains("сейчас: Stage 3"), "{json}");
        assert!(render("", &Cx::default()).is_err());
    }
}
