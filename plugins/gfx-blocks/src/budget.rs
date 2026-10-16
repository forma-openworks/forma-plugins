//! ` ```budget ` — бюджет полигонов модели: из чего складываются треугольники и
//! помещаются ли они в бюджет.
//!
//! ```text
//! Бюджет: 8000 tris
//! Фюзеляж: 4200
//! Крылья: 2100
//! Шасси: 900
//! Кабина: 700
//! ```
//!
//! Первая строка с «Бюджет», «Budget» или «target» — сколько можно; остальные — части.
//! Рисуется одна полоса из частей (часть целого) и метка бюджета на ней, под ней —
//! таблица с числами и долями: цвет никогда не единственный способ узнать часть. Частей
//! больше шести — самые мелкие складываются в «Остальное» (цвета палитры не повторяются
//! и не выдумываются).

use forma_plugin_api::ui::{self, Align, Element, Shape, Tone};

use crate::util::{Cx, key_value, number};

/// Сколько частей получают свой цвет; остальные — в «Остальное».
const MAX_PARTS: usize = 6;

#[derive(Debug, Default, PartialEq)]
pub struct Budget {
    pub target: Option<f64>,
    pub unit: String,
    pub parts: Vec<(String, f64)>,
    pub problems: Vec<String>,
}

pub fn parse(source: &str) -> Budget {
    let mut b = Budget { unit: "tris".into(), ..Default::default() };
    for line in source.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("//")) {
        let Some((key, value)) = key_value(line.trim_start_matches(['-', '*']).trim()) else {
            b.problems.push(line.to_owned());
            continue;
        };
        let Some(n) = number(&value) else {
            b.problems.push(line.to_owned());
            continue;
        };
        if matches!(key.as_str(), "бюджет" | "budget" | "target" | "цель" | "лимит" | "limit") {
            b.target = Some(n);
            // Слово после числа — единица: tris, verts, polys.
            if let Some(unit) = value.split_whitespace().last().filter(|w| w.chars().all(char::is_alphabetic)) {
                b.unit = unit.to_owned();
            }
        } else {
            let name = line.trim_start_matches(['-', '*']).trim().split_once(':').map_or(key.clone(), |(n, _)| n.trim().to_owned());
            b.parts.push((name, n.max(0.0)));
        }
    }
    b
}

/// Части в порядке записи (цвет держится за часть, а не за её место по размеру); если их
/// больше шести, самые мелкие складываются в «Остальное».
fn fold(parts: &[(String, f64)], other: &str) -> Vec<(String, f64)> {
    if parts.len() <= MAX_PARTS {
        return parts.to_vec();
    }
    let mut by_size: Vec<usize> = (0..parts.len()).collect();
    by_size.sort_by(|&a, &b| parts[b].1.total_cmp(&parts[a].1));
    let keep: Vec<usize> = by_size[..MAX_PARTS - 1].to_vec();
    let mut out: Vec<(String, f64)> = (0..parts.len()).filter(|i| keep.contains(i)).map(|i| parts[i].clone()).collect();
    let rest: f64 = (0..parts.len()).filter(|i| !keep.contains(i)).map(|i| parts[i].1).sum();
    out.push((other.to_owned(), rest));
    out
}

const WIDTH: f32 = 640.0;
const BAR_Y: f32 = 26.0;
const BAR_H: f32 = 22.0;
const PAD_X: f32 = 8.0;

pub fn render(source: &str, cx: &Cx) -> Result<Element, String> {
    let b = parse(source);
    if b.parts.is_empty() {
        return Err(cx.tr("нет частей: «Бюджет: 8000 tris», потом «Фюзеляж: 4200»", "no parts: “Budget: 8000 tris”, then “Fuselage: 4200”").into());
    }
    let other = cx.tr("Остальное", "Other");
    let parts = fold(&b.parts, other);
    let total: f64 = parts.iter().map(|p| p.1).sum();
    let scale_max = b.target.unwrap_or(total).max(total).max(1.0) * 1.04;
    let px = |v: f64| PAD_X + (v / scale_max) as f32 * (WIDTH - 2.0 * PAD_X);
    let palette = cx.palette();
    let color_of = |i: usize, name: &str| if name == other && i == parts.len() - 1 && b.parts.len() > MAX_PARTS { "muted".to_owned() } else { palette[i % palette.len()].to_owned() };
    let mut shapes = Vec::new();
    // Дорожка полосы — весь масштаб, еле заметно.
    shapes.push(Shape::Rect { x: PAD_X, y: BAR_Y, w: WIDTH - 2.0 * PAD_X, h: BAR_H, fill: Some("#80808014".into()), stroke: None, stroke_width: 1.0, radius: vec![4.0], tooltip: String::new() });
    let mut acc = 0.0;
    let gap = 2.0;
    let n = parts.len();
    for (i, (name, v)) in parts.iter().enumerate() {
        let (x0, x1) = (px(acc), px(acc + v));
        acc += v;
        // Зазор 2 точки между частями; скруглены только концы полосы.
        let w = (x1 - x0 - if i + 1 < n { gap } else { 0.0 }).max(1.0);
        let radius = match (i == 0, i + 1 == n) {
            (true, true) => vec![4.0],
            (true, false) => vec![4.0, 0.0, 4.0, 0.0],
            (false, true) => vec![0.0, 4.0, 0.0, 4.0],
            _ => Vec::new(),
        };
        let share = if total > 0.0 { v / total * 100.0 } else { 0.0 };
        let tooltip = format!("{name}: {} {} · {share:.0} %", cx.int(*v as i64), b.unit);
        shapes.push(Shape::Rect { x: x0, y: BAR_Y, w, h: BAR_H, fill: Some(color_of(i, name)), stroke: None, stroke_width: 1.0, radius, tooltip });
    }
    // Метка бюджета: тонкая черта поперёк полосы и подпись над ней.
    if let Some(t) = b.target {
        let x = px(t);
        shapes.push(ui::line([x, BAR_Y - 6.0], [x, BAR_Y + BAR_H + 6.0], "heading", 2.0));
        let label = if cx.english { format!("Budget {} {}", cx.int(t as i64), b.unit) } else { format!("Бюджет {} {}", cx.int(t as i64), b.unit) };
        let align = if x > WIDTH * 0.75 { Align::End } else { Align::Center };
        shapes.push(Shape::Text { x, y: BAR_Y - 8.0, text: label, size: 12.0, color: "muted".into(), align, valign: Align::End, bold: false, mono: false });
    }
    // Итог — у конца заполненной части.
    let end = px(total);
    shapes.push(Shape::Text { x: end.min(WIDTH - PAD_X), y: BAR_Y + BAR_H + 8.0, text: format!("{} {}", cx.int(total as i64), b.unit), size: 12.0, color: "text".into(), align: if end > WIDTH * 0.8 { Align::End } else { Align::Center }, valign: Align::Start, bold: true, mono: false });
    let canvas = ui::canvas(WIDTH, BAR_Y + BAR_H + 26.0, shapes);

    // Таблица — легенда и точные числа (цвет — не единственный признак части).
    let base = b.target.unwrap_or(total).max(1.0);
    let mut rows: Vec<Vec<String>> = parts
        .iter()
        .enumerate()
        .map(|(i, (name, v))| vec![format!("■ {name}"), cx.int(*v as i64), format!("{:.0} %", v / base * 100.0), color_of(i, name)])
        .collect();
    let mut table = Vec::new();
    for r in rows.drain(..) {
        // Цветной квадрат — отдельный холст в строке: сама подпись остаётся цветом текста.
        table.push(ui::row([
            ui::canvas(12.0, 12.0, vec![ui::rect(0.0, 1.0, 10.0, 10.0, r[3].clone(), 2.0)]),
            Element::Label { text: r[0].trim_start_matches("■ ").to_owned(), icon: None, tone: Tone::Normal, size: ui::Size::Small, bold: false },
            Element::Label { text: format!("{} · {}", r[1], r[2]), icon: None, tone: Tone::Muted, size: ui::Size::Small, bold: false },
        ]));
    }
    let mut out = vec![canvas];
    out.extend(table);
    // Итог словами: статус — значком и текстом, не одним цветом.
    if let Some(t) = b.target {
        let diff = total - t;
        let status = if diff > 0.0 {
            Element::Label {
                text: if cx.english { format!("Over budget by {} {} ({:.0} %)", cx.int(diff as i64), b.unit, diff / t * 100.0) } else { format!("Бюджет превышен на {} {} ({:.0} %)", cx.int(diff as i64), b.unit, diff / t * 100.0) },
                icon: Some("error".into()),
                tone: Tone::Danger,
                size: ui::Size::Small,
                bold: true,
            }
        } else {
            Element::Label {
                text: if cx.english { format!("Within budget: {} {} left", cx.int(-diff as i64), b.unit) } else { format!("В бюджете: осталось {} {}", cx.int(-diff as i64), b.unit) },
                icon: Some("success".into()),
                tone: Tone::Success,
                size: ui::Size::Small,
                bold: true,
            }
        };
        out.push(status);
    }
    for p in &b.problems {
        out.push(crate::util::warning(if cx.english { format!("Not a number: {p}") } else { format!("Не число: {p}") }));
    }
    Ok(Element::Column { children: out, gap: Some(4.0) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_reads_target_unit_and_parts() {
        let b = parse("Бюджет: 8 000 verts\n- Фюзеляж: 4200\nКрылья: 2 100\nШасси: много\n");
        assert_eq!((b.target, b.unit.as_str()), (Some(8000.0), "verts"));
        assert_eq!(b.parts, vec![("Фюзеляж".to_owned(), 4200.0), ("Крылья".to_owned(), 2100.0)]);
        assert_eq!(b.problems, ["Шасси: много"]);
    }

    #[test]
    fn many_parts_fold_into_other_and_keep_their_order() {
        let parts: Vec<(String, f64)> = (1..=8).map(|i| (format!("p{i}"), i as f64)).collect();
        let f = fold(&parts, "Остальное");
        assert_eq!(f.len(), MAX_PARTS);
        assert_eq!(f.iter().map(|p| p.0.as_str()).collect::<Vec<_>>(), ["p4", "p5", "p6", "p7", "p8", "Остальное"]);
        assert_eq!(f[5].1, 1.0 + 2.0 + 3.0);
    }

    #[test]
    fn over_and_under_budget_say_so_in_words() {
        let over = render("Budget: 1000\nA: 700\nB: 500", &Cx::default()).unwrap().to_json();
        assert!(over.contains("Бюджет превышен на 200 tris (20 %)") && over.contains(r#""icon":"error""#), "{over}");
        let under = render("target: 1000\nA: 700", &Cx { english: true, ..Default::default() }).unwrap().to_json();
        assert!(under.contains("Within budget: 300 tris left"), "{under}");
        assert!(render("Бюджет: 10", &Cx::default()).is_err());
    }
}
