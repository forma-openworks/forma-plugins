//! ` ```storyboard ` — раскадровка: кадры карточками, у каждого картинка, длительность,
//! камера и что происходит. Сверху — сколько кадров и сколько всё длится.
//!
//! ```text
//! ## Взлёт · 4s
//! board/01.png
//! Камера: низко у полосы, наезд
//! Шум двигателей нарастает, МиГ отрывается от полосы.
//!
//! ## Набор высоты · 3s
//! Камера: облёт справа
//! ```
//!
//! Кадр начинается с `##`; после `·` или `|` — длительность (`4s`, `4 с`, `0:04`).
//! Строка-путь к картинке (png, jpg, webp) — картинка кадра; «Камера:» или «Camera:» —
//! камера; остальное — описание (markdown).

use forma_plugin_api::ui::{self, Element, Shape, Tone};

use crate::util::{Cx, number};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shot {
    pub title: String,
    /// Длительность в секундах.
    pub seconds: Option<f64>,
    pub image: Option<String>,
    pub camera: String,
    pub text: String,
}

/// «4s», «4 с», «0:04», «1:30» → секунды.
pub fn seconds(text: &str) -> Option<f64> {
    let t = text.trim();
    if let Some((m, s)) = t.split_once(':') {
        return Some(number(m)? * 60.0 + number(s)?);
    }
    number(t)
}

fn is_image(line: &str) -> bool {
    let l = line.to_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".gif"].iter().any(|e| l.ends_with(e)) && !l.contains(' ')
}

pub fn parse(source: &str) -> Vec<Shot> {
    let mut shots: Vec<Shot> = Vec::new();
    for line in source.lines() {
        let t = line.trim();
        if let Some(head) = t.strip_prefix("##") {
            let head = head.trim_start_matches('#').trim();
            let (title, dur) = match head.rsplit_once('·').or_else(|| head.rsplit_once('|')) {
                Some((a, b)) if seconds(b).is_some() => (a.trim(), seconds(b)),
                _ => (head, None),
            };
            shots.push(Shot { title: title.to_owned(), seconds: dur, ..Default::default() });
            continue;
        }
        let Some(shot) = shots.last_mut() else {
            if !t.is_empty() {
                shots.push(Shot { text: t.to_owned(), ..Default::default() });
            }
            continue;
        };
        if t.is_empty() {
            continue;
        }
        let lower = t.to_lowercase();
        if shot.image.is_none() && is_image(t) {
            shot.image = Some(t.trim_start_matches("![[").trim_end_matches("]]").to_owned());
        } else if let Some(cam) = ["камера:", "camera:", "cam:"].iter().find_map(|k| lower.starts_with(k).then(|| t[k.len()..].trim())) {
            shot.camera = cam.to_owned();
        } else {
            if !shot.text.is_empty() {
                shot.text.push('\n');
            }
            shot.text.push_str(t);
        }
    }
    shots
}

/// Ширина карточки кадра: три в ряд в обычной колонке заметки.
const CARD: f32 = 232.0;
const FRAME_H: f32 = CARD * 9.0 / 16.0 - 18.0;

fn duration(total: f64) -> String {
    let s = total.round() as i64;
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn render(source: &str, cx: &Cx) -> Result<Element, String> {
    let shots = parse(source);
    if shots.is_empty() {
        return Err(cx.tr("нет кадров: кадр начинается с «## Название · 4s»", "no shots: a shot starts with “## Title · 4s”").into());
    }
    let total: f64 = shots.iter().filter_map(|s| s.seconds).sum();
    let mut summary = if cx.english { format!("{} shots", shots.len()) } else { format!("Кадров: {}", shots.len()) };
    if total > 0.0 {
        summary.push_str(&format!(" · {}", duration(total)));
    }
    let inner = CARD - 22.0;
    let cards: Vec<Element> = shots
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut children = Vec::new();
            match &s.image {
                Some(path) => children.push(Element::Image { path: path.clone(), width: Some(inner), alt: s.title.clone() }),
                None => children.push(ui::canvas(
                    inner,
                    FRAME_H,
                    vec![
                        Shape::Rect { x: 0.5, y: 0.5, w: inner - 1.0, h: FRAME_H - 1.0, fill: Some("#80808014".into()), stroke: Some("border".into()), stroke_width: 1.0, radius: vec![6.0], tooltip: String::new() },
                        Shape::Text { x: inner / 2.0, y: FRAME_H / 2.0, text: cx.tr("нет кадра", "no frame").into(), size: 12.0, color: "faint".into(), align: ui::Align::Center, valign: ui::Align::Center, bold: false, mono: false },
                    ],
                )),
            }
            let mut meta = vec![Element::Badge { text: format!("#{}", i + 1), tone: Tone::Muted }];
            if let Some(sec) = s.seconds {
                meta.push(Element::Badge { text: if cx.english { format!("{sec} s") } else { format!("{sec} с") }, tone: Tone::Accent });
            }
            children.push(ui::row(meta));
            if !s.camera.is_empty() {
                children.push(Element::Label { text: s.camera.clone(), icon: Some("show".into()), tone: Tone::Muted, size: ui::Size::Small, bold: false });
            }
            if !s.text.is_empty() {
                children.push(ui::markdown(s.text.clone()));
            }
            let title = if s.title.is_empty() { format!("{}", i + 1) } else { s.title.clone() };
            ui::card(title, CARD, children)
        })
        .collect();
    Ok(ui::column([ui::hint(summary), ui::row(cards)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: &str = "## Взлёт · 4s\nboard/01.png\nКамера: низко у полосы\nШум двигателей нарастает.\nМиГ отрывается.\n\n## Набор высоты | 0:03\nCamera: облёт справа\n\n## Финал\n";

    #[test]
    fn shots_read_title_time_image_camera_and_text() {
        let s = parse(BOARD);
        assert_eq!(s.len(), 3);
        assert_eq!((s[0].title.as_str(), s[0].seconds, s[0].image.as_deref()), ("Взлёт", Some(4.0), Some("board/01.png")));
        assert_eq!((s[0].camera.as_str(), s[0].text.as_str()), ("низко у полосы", "Шум двигателей нарастает.\nМиГ отрывается."));
        assert_eq!((s[1].seconds, s[1].camera.as_str()), (Some(3.0), "облёт справа"));
        assert_eq!(s[2].seconds, None);
        assert_eq!(seconds("1:30"), Some(90.0));
    }

    #[test]
    fn board_sums_up_the_running_time() {
        let json = render(BOARD, &Cx::default()).unwrap().to_json();
        assert!(json.contains("Кадров: 3 · 0:07") && json.contains("нет кадра") && json.contains(r#""width":232.0"#), "{json}");
    }
}
