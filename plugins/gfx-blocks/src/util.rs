//! Общее для блоков: с чем рисуем (тема, язык), разбор строк «ключ: значение» и чисел,
//! палитры.

use forma_plugin_api::ui::Element;
use forma_plugin_api::ui::{self, Tone};

/// С чем рисуется блок: тёмная ли тема, английский ли интерфейс.
#[derive(Clone, Debug, Default)]
pub struct Cx {
    pub dark: bool,
    pub english: bool,
}

impl Cx {
    /// Контекст блока (JSON от программы) и язык интерфейса.
    pub fn new(context: &str, english: bool) -> Self {
        let v: serde_json::Value = serde_json::from_str(context).unwrap_or_default();
        Cx { dark: v["dark"].as_bool().unwrap_or(true), english }
    }

    /// Текст на языке интерфейса.
    pub fn tr<'a>(&self, ru: &'a str, en: &'a str) -> &'a str {
        if self.english { en } else { ru }
    }

    /// Целое с разделителем разрядов: «8 000» или «8,000».
    pub fn int(&self, n: i64) -> String {
        let digits = n.unsigned_abs().to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i).is_multiple_of(3) {
                out.push(if self.english { ',' } else { '\u{202f}' });
            }
            out.push(c);
        }
        if n < 0 { format!("−{out}") } else { out }
    }

    /// Категориальная палитра для темы (те же восемь оттенков, свои шаги для светлой и
    /// тёмной темы; порядок — часть проверки на различимость, не менять).
    pub fn palette(&self) -> &'static [&'static str; 8] {
        if self.dark { &CATEGORICAL_DARK } else { &CATEGORICAL_LIGHT }
    }
}

pub const CATEGORICAL_LIGHT: [&str; 8] = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"];
pub const CATEGORICAL_DARK: [&str; 8] = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"];

/// «ключ: значение» → (ключ в нижнем регистре, значение). Двоеточие — первое в строке.
pub fn key_value(line: &str) -> Option<(String, String)> {
    let (k, v) = line.split_once(':')?;
    let k = k.trim().to_lowercase();
    if k.is_empty() {
        return None;
    }
    Some((k, v.trim().to_owned()))
}

/// Число из текста: пробелы между разрядами, запятая вместо точки и слово после числа
/// («8 000 tris», «0,7», «4s») не мешают.
pub fn number(text: &str) -> Option<f64> {
    let mut s = String::new();
    let mut started = false;
    for c in text.trim().chars() {
        match c {
            '0'..='9' => {
                s.push(c);
                started = true;
            }
            '.' | ',' if started => s.push('.'),
            '-' | '−' if !started && s.is_empty() => s.push('-'),
            ' ' | '\u{a0}' | '\u{202f}' | '\u{2009}' | '_' if started => {}
            _ if started => break,
            _ => {}
        }
    }
    s.trim_end_matches('.').parse().ok()
}

/// Текст не шире `width`: лишнее — многоточием. Ширина — на глаз: шрифт программы
/// пропорциональный, средняя буква — чуть больше половины кегля.
pub fn fit(text: &str, width: f32, size: f32) -> String {
    let max = (width / (size * 0.56)).floor().max(1.0) as usize;
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Пометка о плохой строке блока: блок рисуется, а строку видно.
pub fn warning(text: impl Into<String>) -> Element {
    Element::Label { text: text.into(), icon: Some("warning".into()), tone: Tone::Warning, size: ui::Size::Small, bold: false }
}

/// Цвет `#rrggbb` → доли 0…1 (sRGB). Непонятное — `None`.
pub fn parse_hex(text: &str) -> Option<[f32; 3]> {
    let hex = text.trim().strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    match hex.len() {
        6 | 8 => Some([byte(0)? as f32 / 255.0, byte(2)? as f32 / 255.0, byte(4)? as f32 / 255.0]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_keys_read_like_people_write_them() {
        assert_eq!(number("8 000 tris"), Some(8000.0));
        assert_eq!(number("0,7"), Some(0.7));
        assert_eq!(number("4s"), Some(4.0));
        assert_eq!(number("−3"), Some(-3.0));
        assert_eq!(number("бюджет"), None);
        assert_eq!(key_value("Base Color: #6b7350"), Some(("base color".into(), "#6b7350".into())));
        assert_eq!(key_value("время: 0:04").unwrap().1, "0:04");
        let ru = Cx::new(r#"{"dark":false}"#, false);
        assert_eq!((ru.int(8000), ru.int(950)), ("8\u{202f}000".into(), "950".into()));
        assert!(!ru.dark);
        assert_eq!(Cx::new("{}", true).int(1234567), "1,234,567");
        assert_eq!(fit("Retopology and cleanup", 60.0, 10.0), "Retopolog…");
        assert_eq!(parse_hex("#ff8000"), Some([1.0, 128.0 / 255.0, 0.0]));
    }
}
