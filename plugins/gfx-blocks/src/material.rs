//! ` ```material ` — карточка PBR-материала: шар с этим материалом и его параметры.
//!
//! ```text
//! name: Камуфляж ВВС
//! base color: #6b7350
//! metallic: 0
//! roughness: 0.7
//! emission: #000000
//! base color map: textures/camo_albedo.png
//! normal map: textures/camo_normal.png
//! ```
//!
//! Шар считается здесь же, по той же модели, что Principled BSDF (metallic/roughness):
//! рассеянный свет по Ламберту, блик GGX со Smith и Френелем Шлика, мягкий свет неба.
//! Ключи можно писать по-русски: «цвет», «металл», «шероховатость», «свечение».

use forma_plugin_api::ui::{self, Align, Element, Shape};

use crate::util::{Cx, fit, key_value, number, parse_hex};

#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    pub base: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub emission: [f32; 3],
    /// Карты: (что это, путь).
    pub maps: Vec<(String, String)>,
    pub problems: Vec<String>,
}

impl Default for Material {
    fn default() -> Self {
        // Значения Principled BSDF по умолчанию.
        Self { name: String::new(), base: [0.8, 0.8, 0.8], metallic: 0.0, roughness: 0.5, emission: [0.0; 3], maps: Vec::new(), problems: Vec::new() }
    }
}

pub fn parse(source: &str) -> Material {
    let mut m = Material::default();
    for line in source.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with("//")) {
        let Some((key, value)) = key_value(line) else {
            m.problems.push(line.to_owned());
            continue;
        };
        let ok = match key.as_str() {
            "name" | "имя" | "название" => {
                m.name = value.clone();
                true
            }
            "base color" | "color" | "albedo" | "цвет" | "базовый цвет" => parse_hex(&value).map(|c| m.base = c).is_some(),
            "metallic" | "metalness" | "металл" | "металличность" => number(&value).map(|v| m.metallic = (v as f32).clamp(0.0, 1.0)).is_some(),
            "roughness" | "шероховатость" => number(&value).map(|v| m.roughness = (v as f32).clamp(0.0, 1.0)).is_some(),
            "emission" | "свечение" => parse_hex(&value).map(|c| m.emission = c).is_some(),
            k if k.ends_with("map") || k.ends_with("карта") || k.starts_with("карта") => {
                let kind = k.trim_end_matches("map").trim_end_matches("карта").trim_start_matches("карта").trim();
                m.maps.push((kind.to_owned(), value.clone()));
                true
            }
            _ => false,
        };
        if !ok {
            m.problems.push(line.to_owned());
        }
    }
    m
}

fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Цвет точки шара с нормалью `n` (взгляд вдоль +z): тот же расчёт, что у Principled BSDF,
/// без отражений окружения — их заменяет мягкое небо.
pub fn shade(m: &Material, n: [f32; 3]) -> [f32; 3] {
    let base = m.base.map(to_linear);
    let v = [0.0, 0.0, 1.0];
    let lights = [(norm([-0.55, 0.65, 0.55]), 3.2), (norm([0.7, -0.2, 0.4]), 0.6)];
    let alpha = (m.roughness * m.roughness).max(0.002);
    let a2 = alpha * alpha;
    let k = (m.roughness + 1.0).powi(2) / 8.0;
    let f0: [f32; 3] = std::array::from_fn(|i| 0.04 * (1.0 - m.metallic) + base[i] * m.metallic);
    let n_v = dot(n, v).max(1e-4);
    let mut out = [0.0f32; 3];
    for (l, power) in lights {
        let n_l = dot(n, l);
        if n_l <= 0.0 {
            continue;
        }
        let h = norm([l[0] + v[0], l[1] + v[1], l[2] + v[2]]);
        let n_h = dot(n, h).max(0.0);
        let v_h = dot(v, h).max(0.0);
        let d = a2 / (std::f32::consts::PI * (n_h * n_h * (a2 - 1.0) + 1.0).powi(2));
        let g = (n_l / (n_l * (1.0 - k) + k)) * (n_v / (n_v * (1.0 - k) + k));
        for i in 0..3 {
            let f = f0[i] + (1.0 - f0[i]) * (1.0 - v_h).powi(5);
            let spec = d * g * f / (4.0 * n_l * n_v + 1e-4);
            let diffuse = (1.0 - f) * (1.0 - m.metallic) * base[i] / std::f32::consts::PI;
            out[i] += (diffuse + spec) * n_l * power;
        }
    }
    // Небо: сверху светлее, снизу темнее; металл отражает его своим цветом.
    let sky = 0.18 + 0.12 * n[1];
    for i in 0..3 {
        let tint = base[i] * m.metallic + (1.0 - m.metallic) * base[i] * 0.6 + 0.04;
        out[i] += sky * tint * (1.0 - 0.5 * m.roughness);
        out[i] += to_linear(m.emission[i]);
        // Мягкое сжатие ярких мест (как Filmic/AgX — без пересвета).
        out[i] = to_srgb(out[i] / (1.0 + out[i] * 0.35));
    }
    out
}

const BALL: f32 = 180.0;
const WIDTH: f32 = 560.0;

/// Шар: кольца вершин от центра к краю, каждая со своим цветом.
fn ball(m: &Material, cx: f32, cy: f32, r: f32) -> Shape {
    let (rings, segs) = (22usize, 48usize);
    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    let push = |x: f32, y: f32, vertices: &mut Vec<[f32; 2]>, colors: &mut Vec<String>| {
        let z = (1.0 - x * x - y * y).max(0.0).sqrt();
        let c = shade(m, [x, -y, z]);
        vertices.push([cx + x * r, cy + y * r]);
        colors.push(ui::rgb(c[0], c[1], c[2]));
    };
    push(0.0, 0.0, &mut vertices, &mut colors);
    for ring in 1..=rings {
        // Кольца гуще к краю: там свет меняется быстрее.
        let t = (ring as f32 / rings as f32).sqrt().min(0.999);
        for s in 0..segs {
            let a = s as f32 / segs as f32 * std::f32::consts::TAU;
            push(t * a.cos(), t * a.sin(), &mut vertices, &mut colors);
        }
    }
    let mut indices = Vec::new();
    for s in 0..segs as u32 {
        indices.extend([0, 1 + s, 1 + (s + 1) % segs as u32]);
    }
    for ring in 1..rings as u32 {
        let (a0, b0) = (1 + (ring - 1) * segs as u32, 1 + ring * segs as u32);
        for s in 0..segs as u32 {
            let s1 = (s + 1) % segs as u32;
            indices.extend([a0 + s, b0 + s, b0 + s1, a0 + s, b0 + s1, a0 + s1]);
        }
    }
    Shape::Mesh { vertices, colors, indices }
}

pub fn render(source: &str, cx: &Cx) -> Result<Element, String> {
    let m = parse(source);
    let r = BALL / 2.0 - 6.0;
    let (bx, by) = (BALL / 2.0, BALL / 2.0);
    let mut shapes = vec![
        // Тень под шаром.
        Shape::Circle { x: bx + 4.0, y: by + 6.0, r, fill: Some("#00000040".into()), stroke: None, stroke_width: 1.0, tooltip: String::new() },
        ball(&m, bx, by, r),
    ];
    // Параметры справа: цвет образцом, числа — полосой и значением.
    let x0 = BALL + 24.0;
    let mut y = 14.0;
    if !m.name.is_empty() {
        shapes.push(Shape::Text { x: x0, y, text: fit(&m.name, WIDTH - x0, 16.0), size: 16.0, color: "heading".into(), align: Align::Start, valign: Align::Start, bold: true, mono: false });
        y += 30.0;
    }
    let label = |shapes: &mut Vec<Shape>, y: f32, text: &str| {
        shapes.push(Shape::Text { x: x0, y: y + 9.0, text: text.to_owned(), size: 12.5, color: "muted".into(), align: Align::Start, valign: Align::Center, bold: false, mono: false });
    };
    let vx = x0 + 120.0;
    let hex = ui::rgb(m.base[0], m.base[1], m.base[2]);
    label(&mut shapes, y, "Base Color");
    shapes.push(Shape::Rect { x: vx, y, w: 36.0, h: 18.0, fill: Some(hex.clone()), stroke: Some("border".into()), stroke_width: 1.0, radius: vec![3.0], tooltip: String::new() });
    shapes.push(Shape::Text { x: vx + 46.0, y: y + 9.0, text: hex, size: 12.0, color: "text".into(), align: Align::Start, valign: Align::Center, bold: false, mono: true });
    y += 28.0;
    for (name, v) in [("Metallic", m.metallic), ("Roughness", m.roughness)] {
        label(&mut shapes, y, name);
        let w = 150.0;
        shapes.push(Shape::Rect { x: vx, y: y + 4.0, w, h: 10.0, fill: Some("#80808026".into()), stroke: None, stroke_width: 1.0, radius: vec![5.0], tooltip: String::new() });
        shapes.push(Shape::Rect { x: vx, y: y + 4.0, w: (w * v).max(2.0), h: 10.0, fill: Some("accent".into()), stroke: None, stroke_width: 1.0, radius: vec![5.0], tooltip: String::new() });
        shapes.push(Shape::Text { x: vx + w + 10.0, y: y + 9.0, text: format!("{v:.2}"), size: 12.0, color: "text".into(), align: Align::Start, valign: Align::Center, bold: false, mono: true });
        y += 26.0;
    }
    if m.emission != [0.0; 3] {
        let hex = ui::rgb(m.emission[0], m.emission[1], m.emission[2]);
        label(&mut shapes, y, "Emission");
        shapes.push(Shape::Rect { x: vx, y, w: 36.0, h: 18.0, fill: Some(hex.clone()), stroke: Some("border".into()), stroke_width: 1.0, radius: vec![3.0], tooltip: String::new() });
        shapes.push(Shape::Text { x: vx + 46.0, y: y + 9.0, text: hex, size: 12.0, color: "text".into(), align: Align::Start, valign: Align::Center, bold: false, mono: true });
        y += 28.0;
    }
    // Карты — миниатюрами с подписью.
    let mut height = BALL.max(y + 6.0);
    if !m.maps.is_empty() {
        let ty = height + 8.0;
        let thumb = 72.0;
        for (i, (kind, path)) in m.maps.iter().enumerate() {
            let x = i as f32 * (thumb + 16.0);
            if x + thumb > WIDTH {
                break;
            }
            shapes.push(Shape::Image { x, y: ty, w: thumb, h: thumb, path: path.clone() });
            shapes.push(Shape::Rect { x, y: ty, w: thumb, h: thumb, fill: None, stroke: Some("border".into()), stroke_width: 1.0, radius: vec![4.0], tooltip: path.clone() });
            let title = if kind.is_empty() { cx.tr("карта", "map").to_owned() } else { kind.clone() };
            shapes.push(Shape::Text { x: x + thumb / 2.0, y: ty + thumb + 6.0, text: fit(&title, thumb + 12.0, 11.5), size: 11.5, color: "muted".into(), align: Align::Center, valign: Align::Start, bold: false, mono: false });
        }
        height = ty + thumb + 24.0;
    }
    let mut out = vec![ui::canvas(WIDTH, height, shapes)];
    for p in &m.problems {
        out.push(crate::util::warning(if cx.english { format!("Not understood: {p}") } else { format!("Не разобрано: {p}") }));
    }
    Ok(ui::column(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_reads_keys_in_both_languages() {
        let m = parse("name: Камуфляж\nцвет: #6b7350\nметалл: 0,2\nroughness: 1.5\nnormal map: t/n.png\nкарта шероховатости: t/r.png\nблеск: да");
        assert_eq!(m.name, "Камуфляж");
        assert_eq!((m.metallic, m.roughness), (0.2, 1.0), "шероховатость не больше 1");
        assert_eq!(m.maps, vec![("normal".to_owned(), "t/n.png".to_owned()), ("шероховатости".to_owned(), "t/r.png".to_owned())]);
        assert_eq!(m.problems, ["блеск: да"]);
    }

    #[test]
    fn the_ball_is_lit_from_the_upper_left_and_shiny_when_smooth() {
        let rough = Material { roughness: 0.9, ..Default::default() };
        let smooth = Material { roughness: 0.1, ..Default::default() };
        let lit = shade(&rough, norm([-0.4, 0.5, 0.75]));
        let dark = shade(&rough, norm([0.5, -0.6, 0.6]));
        assert!(lit[0] > dark[0], "сверху слева светлее");
        // Блик гладкого — ярче и острее: в направлении половинного вектора.
        let h = norm([-0.55 / 2.0, 0.65 / 2.0, (0.55 + 1.0) / 2.0]);
        assert!(shade(&smooth, h)[0] > shade(&rough, h)[0]);
        let Shape::Mesh { vertices, colors, indices } = ball(&rough, 90.0, 90.0, 80.0) else { panic!() };
        assert_eq!(vertices.len(), colors.len());
        assert!(indices.iter().all(|&i| (i as usize) < vertices.len()) && indices.len() % 3 == 0);
    }
}
