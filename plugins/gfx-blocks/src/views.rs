//! ` ```gltf ` — модель из хранилища: её можно вращать, рядом сведения (треугольники,
//! вершины, материалы, UV, размеры). ` ```uv ` — UV-раскладка той же модели, как в
//! UV-редакторе Blender: квадрат 0–1, грани и рёбра, сколько островов и места занято.
//!
//! ```text
//! model: models/mig-21.glb
//! wireframe: true
//! color: #b0b0b0
//! ```
//!
//! У `uv` можно указать меш: `mesh: Fuselage` — тогда только его раскладка.

use std::collections::HashMap;

use forma_plugin_api::ui::{self, Element, Shape, Tone};

use crate::model::{self, Model};
use crate::util::{Cx, key_value, parse_hex};

#[derive(Debug, Default, PartialEq)]
struct Options {
    model: String,
    mesh: Option<String>,
    wireframe: bool,
    color: Option<String>,
}

fn options(source: &str) -> Options {
    let mut o = Options::default();
    for line in source.lines() {
        match key_value(line) {
            Some((k, v)) if matches!(k.as_str(), "model" | "file" | "модель" | "файл") => o.model = v.trim_start_matches("![[").trim_end_matches("]]").to_owned(),
            Some((k, v)) if matches!(k.as_str(), "mesh" | "меш") => o.mesh = Some(v),
            Some((k, v)) if matches!(k.as_str(), "wireframe" | "рёбра" | "ребра" | "сетка") => o.wireframe = matches!(v.to_lowercase().as_str(), "true" | "yes" | "on" | "да" | "1"),
            Some((k, v)) if matches!(k.as_str(), "color" | "цвет") && parse_hex(&v).is_some() => o.color = Some(v),
            _ if o.model.is_empty() && (line.trim().to_lowercase().ends_with(".glb") || line.trim().to_lowercase().ends_with(".gltf")) => o.model = line.trim().to_owned(),
            _ => {}
        }
    }
    o
}

fn load(o: &Options, cx: &Cx, read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Model, String> {
    if o.model.is_empty() {
        return Err(cx.tr("нет модели: «model: models/plane.glb»", "no model: “model: models/plane.glb”").into());
    }
    model::load(&o.model, read)
}

fn badge(text: String, tone: Tone) -> Element {
    Element::Badge { text, tone }
}

pub fn render_gltf(source: &str, cx: &Cx, read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Element, String> {
    let o = options(source);
    let m = load(&o, cx, read)?;
    let indices = m.preview_indices();
    let shown = indices.len() / 3;
    let view = Element::View3d { positions: m.positions.clone(), indices, color: o.color.clone(), height: 340.0, wireframe: o.wireframe };
    let en = cx.english;
    let mut facts = vec![
        badge(format!("{} {}", cx.int(m.triangles() as i64), if en { "tris" } else { "треуг." }), Tone::Accent),
        badge(format!("{} {}", cx.int(m.vertices() as i64), if en { "verts" } else { "вершин" }), Tone::Muted),
        badge(format!("{} {}", m.materials, if en { "materials" } else { "матер." }), Tone::Muted),
        badge(format!("{} {}", m.mesh_names.len(), if en { "meshes" } else { "мешей" }), Tone::Muted),
    ];
    facts.push(if m.has_uv { badge("UV ✓".into(), Tone::Success) } else { badge(cx.tr("нет UV", "no UV").into(), Tone::Warning) });
    if !m.has_normals {
        facts.push(badge(cx.tr("нет нормалей", "no normals").into(), Tone::Warning));
    }
    let [x, y, z] = m.size();
    let mut hint = if en { format!("{} · size {x:.2} × {y:.2} × {z:.2}", o.model) } else { format!("{} · размер {x:.2} × {y:.2} × {z:.2}", o.model) };
    if shown < m.triangles() {
        hint.push_str(&if en { format!(" · showing {} of the triangles", cx.int(shown as i64)) } else { format!(" · показано {} треугольников", cx.int(shown as i64)) });
    }
    if m.skipped > 0 {
        hint.push_str(&if en { format!(" · {} non-triangle parts skipped", m.skipped) } else { format!(" · пропущено частей не из треугольников: {}", m.skipped) });
    }
    hint.push_str(cx.tr(" · тянуть — вращать, Ctrl и колесо — приблизить", " · drag to orbit, Ctrl+wheel to zoom"));
    Ok(ui::column([view, ui::row(facts), ui::hint(hint)]))
}

/// Сведения о раскладке.
#[derive(Debug, Default, PartialEq)]
pub struct UvStats {
    pub triangles: usize,
    pub islands: usize,
    /// Доля квадрата 0–1 под гранями (перекрытия считаются дважды).
    pub coverage: f32,
    /// Вершины за пределами квадрата 0–1.
    pub outside: usize,
}

/// Острова: треугольники, связанные общими UV-точками (одинаковые координаты — одна точка,
/// как у «Select Linked» в UV-редакторе).
pub fn uv_stats(uvs: &[[f32; 2]], tris: &[[u32; 3]]) -> UvStats {
    let key = |p: [f32; 2]| ((p[0] * 4096.0).round() as i64, (p[1] * 4096.0).round() as i64);
    let mut point_id: HashMap<(i64, i64), usize> = HashMap::new();
    let ids: Vec<usize> = uvs.iter().map(|p| {
        let n = point_id.len();
        *point_id.entry(key(*p)).or_insert(n)
    }).collect();
    let mut parent: Vec<usize> = (0..point_id.len()).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut coverage = 0.0f32;
    for t in tris {
        let [a, b, c] = t.map(|i| ids[i as usize]);
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        parent[ra] = rb;
        let (rb, rc) = (find(&mut parent, b), find(&mut parent, c));
        parent[rb] = rc;
        let [p, q, r] = t.map(|i| uvs[i as usize]);
        coverage += ((q[0] - p[0]) * (r[1] - p[1]) - (r[0] - p[0]) * (q[1] - p[1])).abs() / 2.0;
    }
    let used: std::collections::HashSet<usize> = tris.iter().flat_map(|t| t.map(|i| ids[i as usize])).collect();
    let roots: std::collections::HashSet<usize> = used.iter().map(|&p| find(&mut parent, p)).collect();
    let outside = uvs.iter().filter(|p| p[0] < -1e-4 || p[0] > 1.0001 || p[1] < -1e-4 || p[1] > 1.0001).count();
    UvStats { triangles: tris.len(), islands: roots.len(), coverage, outside }
}

/// Размер квадрата UV на холсте.
const SIDE: f32 = 440.0;
const PAD: f32 = 12.0;
/// Больше рёбер не рисуем по одному: заливка граней остаётся, рёбра — нет.
const MAX_EDGES: usize = 40_000;

pub fn render_uv(source: &str, cx: &Cx, read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Element, String> {
    let o = options(source);
    let m = load(&o, cx, read)?;
    if !m.has_uv && m.uvs.iter().all(|v| *v == 0.0) {
        return Err(cx.tr("у модели нет UV", "the model has no UVs").into());
    }
    let mesh_filter = match &o.mesh {
        Some(name) => Some(m.mesh_names.iter().position(|n| n.eq_ignore_ascii_case(name)).ok_or_else(|| format!("{}: {name}", cx.tr("нет меша", "no mesh")))? as u32),
        None => None,
    };
    let uvs: Vec<[f32; 2]> = m.uvs.as_chunks::<2>().0.to_vec();
    let tris: Vec<[u32; 3]> = m.indices.as_chunks::<3>().0.iter().zip(&m.tri_mesh).filter(|(_, mesh)| mesh_filter.is_none_or(|f| **mesh == f)).map(|(t, _)| *t).collect();
    let stats = uv_stats(&uvs, &tris);
    // UV: v вверх, а на холсте y вниз.
    let at = |p: [f32; 2]| [PAD + p[0] * SIDE, PAD + (1.0 - p[1]) * SIDE];
    let size = SIDE + 2.0 * PAD;
    let mut shapes = vec![Shape::Rect { x: 0.0, y: 0.0, w: size, h: size, fill: Some("#232323".into()), stroke: None, stroke_width: 1.0, radius: vec![6.0], tooltip: String::new() }];
    for k in 0..=8 {
        let t = PAD + k as f32 * SIDE / 8.0;
        let c = if k % 4 == 0 { "#4a4a4a" } else { "#333333" };
        shapes.push(ui::line([PAD, t], [PAD + SIDE, t], c, 1.0));
        shapes.push(ui::line([t, PAD], [t, PAD + SIDE], c, 1.0));
    }
    // Грани — еле заметной заливкой, как у Blender (#ffffff0a), рёбра — светло-серым.
    let used: Vec<u32> = {
        let mut v: Vec<u32> = tris.iter().flatten().copied().collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let remap: HashMap<u32, u32> = used.iter().enumerate().map(|(i, &v)| (v, i as u32)).collect();
    shapes.push(Shape::Mesh {
        vertices: used.iter().map(|&i| at(uvs[i as usize])).collect(),
        colors: vec!["#ffffff14".into()],
        indices: tris.iter().flat_map(|t| t.map(|i| remap[&i])).collect(),
    });
    let mut edges: Vec<(u32, u32)> = tris.iter().flat_map(|t| [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])]).map(|(a, b)| (a.min(b), a.max(b))).collect();
    edges.sort_unstable();
    edges.dedup();
    let drawn = edges.len() <= MAX_EDGES;
    if drawn {
        for (a, b) in &edges {
            shapes.push(ui::line(at(uvs[*a as usize]), at(uvs[*b as usize]), "#c0c0c0", 0.6));
        }
    }
    shapes.push(Shape::Rect { x: PAD, y: PAD, w: SIDE, h: SIDE, fill: None, stroke: Some("#707070".into()), stroke_width: 1.0, radius: Vec::new(), tooltip: String::new() });
    let mut out = vec![ui::canvas(size, size, shapes)];
    let en = cx.english;
    let mut facts = vec![
        badge(format!("{} {}", stats.islands, if en { "islands" } else { "островов" }), Tone::Accent),
        badge(format!("≈ {:.0} % {}", (stats.coverage * 100.0).min(999.0), if en { "of the square" } else { "квадрата" }), Tone::Muted),
        badge(format!("{} {}", cx.int(stats.triangles as i64), if en { "tris" } else { "треуг." }), Tone::Muted),
    ];
    if let Some(name) = &o.mesh {
        facts.insert(0, badge(name.clone(), Tone::Normal));
    }
    out.push(ui::row(facts));
    if stats.outside > 0 {
        out.push(crate::util::warning(if en { format!("{} UV points outside 0–1 (UDIM or a mistake?)", stats.outside) } else { format!("Точек UV за квадратом 0–1: {} (UDIM или ошибка?)", stats.outside) }));
    }
    if !drawn {
        out.push(ui::hint(cx.tr("Рёбер слишком много — показана только заливка граней.", "Too many edges — only the faces are shown.")));
    }
    out.push(ui::hint(cx.tr("Площадь считает перекрытия дважды: больше 100 % — острова лежат друг на друге.", "Overlaps count twice: over 100 % means islands overlap.")));
    Ok(Element::Column { children: out, gap: Some(6.0) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tests::quad_gltf;

    fn read(p: &str) -> Result<Vec<u8>, String> {
        if p == "m/quad.gltf" { Ok(quad_gltf().into_bytes()) } else { Err(format!("{p}: missing")) }
    }

    #[test]
    fn options_read_model_mesh_and_wireframe() {
        let o = options("model: ![[models/mig.glb]]\nmesh: Fuselage\nwireframe: да\ncolor: #a0a0a0");
        assert_eq!(o, Options { model: "models/mig.glb".into(), mesh: Some("Fuselage".into()), wireframe: true, color: Some("#a0a0a0".into()) });
        assert_eq!(options("models/plane.gltf").model, "models/plane.gltf");
    }

    #[test]
    fn gltf_block_shows_the_model_and_its_facts() {
        let json = render_gltf("model: m/quad.gltf", &Cx::default(), &read).unwrap().to_json();
        assert!(json.contains(r#""type":"view3d""#) && json.contains("2 треуг.") && json.contains("UV ✓") && json.contains("нет нормалей"), "{json}");
        assert!(render_gltf("model: m/x.glb", &Cx::default(), &read).is_err());
    }

    #[test]
    fn uv_islands_and_coverage() {
        // Два треугольника квадрата — один остров на четверть площади 0–1.
        let json = render_uv("model: m/quad.gltf", &Cx::default(), &read).unwrap().to_json();
        assert!(json.contains("1 островов") && json.contains("≈ 25 %"), "{json}");
        assert!(render_uv("model: m/quad.gltf\nmesh: Tail", &Cx::default(), &read).is_err());
        // Два отдельных треугольника и точка за квадратом.
        let s = uv_stats(&[[0.0, 0.0], [0.2, 0.0], [0.0, 0.2], [0.5, 0.5], [0.7, 0.5], [1.5, 0.7]], &[[0, 1, 2], [3, 4, 5]]);
        assert_eq!((s.islands, s.outside), (2, 1));
    }
}
