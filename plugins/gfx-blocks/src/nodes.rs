//! ` ```nodes ` — граф нод, как редактор нод Blender: шапка ноды цветом её класса, сокеты
//! цветом типа, связи — кривые от выхода ко входу.
//!
//! ```text
//! Image Texture [texture]
//!   < Color: color
//!   < Alpha: float
//!   > Vector: vector
//! Principled BSDF [shader]
//!   < BSDF: shader
//!   > Base Color: color
//!   > Roughness: float = 0.5
//! Material Output [output]
//!   > Surface: shader
//! Image Texture.Color -> Principled BSDF.Base Color
//! Principled BSDF.BSDF -> Material Output.Surface
//! ```
//!
//! Нода — строка без отступа (класс в скобках: input, output, shader, texture, color,
//! vector, converter, group, geometry, attribute, filter, script, matte, distort). Сокеты —
//! строки с отступом: `<` выход, `>` вход, после двоеточия тип (float, int, vector, color,
//! shader, bool, string, geometry, object, image, texture, material, rotation, matrix,
//! menu, collection) и значение после `=`. Связь — `Нода.Выход -> Нода.Вход`. Ноды
//! расставляются сами, столбцами слева направо по связям.
//!
//! Цвета и размеры — из исходников Blender 5.3: `std_node_socket_colors` (drawnode.cc),
//! тема по умолчанию (`userdef_default_theme.c`), `NODE_DY`, `NODE_SOCKSIZE`.

use forma_plugin_api::ui::{self, Align, Element, Shape};

use crate::util::{Cx, fit};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Socket {
    pub name: String,
    pub kind: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Node {
    pub name: String,
    pub class: String,
    pub outputs: Vec<Socket>,
    pub inputs: Vec<Socket>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub from: (usize, usize),
    pub to: (usize, usize),
}

#[derive(Debug, Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    /// Строки, которые не разобрались (показываются под графом).
    pub problems: Vec<String>,
}

/// Цвет сокета по типу (Blender 5.3, `std_node_socket_colors`).
pub fn socket_color(kind: &str) -> &'static str {
    match kind.to_lowercase().as_str() {
        "float" | "value" | "number" => "#a1a1a1",
        "vector" | "vec" => "#6363c7",
        "color" | "rgba" | "colour" => "#c7c729",
        "shader" | "bsdf" | "closure" => "#63c763",
        "bool" | "boolean" => "#cca6d6",
        "int" | "integer" => "#598c5c",
        "string" | "text" => "#70b3ff",
        "object" => "#ed9e5c",
        "image" => "#633863",
        "geometry" | "geo" => "#00d6a3",
        "collection" => "#f5f5f5",
        "texture" => "#9e4fa3",
        "material" => "#eb7582",
        "rotation" => "#a663c7",
        "matrix" => "#b83385",
        "menu" => "#666666",
        _ => "#a1a1a1",
    }
}

/// Цвет шапки ноды по классу (тема Blender 5.3 по умолчанию).
pub fn header_color(class: &str) -> &'static str {
    match class.to_lowercase().as_str() {
        "input" => "#82354c",
        "output" => "#3e232a",
        "shader" => "#2b652b",
        "texture" => "#79461d",
        "color" => "#6e6e23",
        "vector" => "#3c3c83",
        "converter" => "#246283",
        "group" => "#374725",
        "geometry" => "#1d725e",
        "attribute" => "#1d2546",
        "filter" => "#412b51",
        "script" => "#203c3c",
        "matte" => "#5a3838",
        "distort" => "#3e5a5b",
        _ => "#4d4d4d",
    }
}

pub fn parse(source: &str) -> Graph {
    let mut g = Graph::default();
    let mut pending_links = Vec::new();
    for raw in source.lines() {
        if raw.trim().is_empty() || raw.trim_start().starts_with("//") {
            continue;
        }
        let indented = raw.starts_with([' ', '\t']);
        let line = raw.trim();
        if line.contains("->") {
            pending_links.push(line.to_owned());
        } else if indented && (line.starts_with('<') || line.starts_with('>')) {
            let Some(node) = g.nodes.last_mut() else {
                g.problems.push(line.to_owned());
                continue;
            };
            let output = line.starts_with('<');
            let rest = line[1..].trim();
            let (name_kind, value) = match rest.split_once('=') {
                Some((a, v)) => (a.trim(), Some(v.trim().to_owned())),
                None => (rest, None),
            };
            let (name, kind) = match name_kind.rsplit_once(':') {
                Some((n, k)) => (n.trim(), k.trim()),
                None => (name_kind, "float"),
            };
            let socket = Socket { name: name.to_owned(), kind: kind.to_lowercase(), value };
            if output { node.outputs.push(socket) } else { node.inputs.push(socket) }
        } else {
            let (name, class) = match (line.rfind('['), line.ends_with(']')) {
                (Some(i), true) => (line[..i].trim(), line[i + 1..line.len() - 1].trim()),
                _ => (line, ""),
            };
            g.nodes.push(Node { name: name.to_owned(), class: class.to_lowercase(), ..Default::default() });
        }
    }
    for l in pending_links {
        let Some((a, b)) = l.split_once("->") else { continue };
        match (find_socket(&g.nodes, a.trim(), true), find_socket(&g.nodes, b.trim(), false)) {
            (Some(from), Some(to)) => g.links.push(Link { from, to }),
            _ => g.problems.push(l),
        }
    }
    g
}

/// `Нода.Сокет` → (номер ноды, номер сокета). Имена нод могут содержать точки, поэтому
/// ищется нода, чьё имя — начало строки перед точкой.
fn find_socket(nodes: &[Node], text: &str, output: bool) -> Option<(usize, usize)> {
    nodes.iter().enumerate().filter(|(_, n)| text.len() > n.name.len() && text.starts_with(&n.name) && text[n.name.len()..].starts_with('.')).max_by_key(|(_, n)| n.name.len()).and_then(|(i, n)| {
        let socket = text[n.name.len() + 1..].trim();
        let list = if output { &n.outputs } else { &n.inputs };
        list.iter().position(|s| s.name.eq_ignore_ascii_case(socket)).map(|j| (i, j))
    })
}

// Размеры — как у Blender при обычном масштабе интерфейса (widget_unit = 20).
const NODE_W: f32 = 150.0;
const HEADER: f32 = 22.0;
const ROW: f32 = 22.0;
const SOCKET_R: f32 = 5.0;
const ROUND: f32 = 4.0;
const COL_GAP: f32 = 70.0;
const NODE_GAP: f32 = 26.0;
const MARGIN: f32 = 24.0;
/// Связь изгибается на 0,4 расстояния по горизонтали (`noodle_curving` 4 у Blender).
const CURVING: f32 = 0.4;

fn node_height(n: &Node) -> f32 {
    HEADER + 6.0 + (n.outputs.len() + n.inputs.len()).max(1) as f32 * ROW + 6.0
}

/// Столбец каждой ноды: самый длинный путь от нод без входящих связей.
fn columns(g: &Graph) -> Vec<usize> {
    let mut col = vec![0usize; g.nodes.len()];
    // Не больше, чем нод: на циклах раскладка просто останавливается.
    for _ in 0..g.nodes.len() {
        let mut changed = false;
        for l in &g.links {
            if l.from.0 != l.to.0 && col[l.to.0] < col[l.from.0] + 1 {
                col[l.to.0] = col[l.from.0] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    col
}

/// Левый верхний угол каждой ноды.
fn layout(g: &Graph) -> (Vec<[f32; 2]>, f32, f32) {
    let col = columns(g);
    let ncols = col.iter().max().map_or(0, |m| m + 1);
    let mut pos = vec![[0.0, 0.0]; g.nodes.len()];
    let mut heights = vec![0.0f32; ncols];
    for (c, column_height) in heights.iter_mut().enumerate() {
        let mut y = MARGIN;
        for (i, n) in g.nodes.iter().enumerate().filter(|(i, _)| col[*i] == c) {
            pos[i] = [MARGIN + c as f32 * (NODE_W + COL_GAP), y];
            y += node_height(n) + NODE_GAP;
        }
        *column_height = y - NODE_GAP + MARGIN;
    }
    // Столбцы покороче — по центру высоты, как раскладывают ноды руками.
    let height = heights.iter().copied().fold(0.0, f32::max);
    for (c, column_height) in heights.iter().enumerate() {
        let shift = (height - column_height) / 2.0;
        for (i, p) in pos.iter_mut().enumerate() {
            if col[i] == c {
                p[1] += shift;
            }
        }
    }
    let width = 2.0 * MARGIN + ncols as f32 * NODE_W + ncols.saturating_sub(1) as f32 * COL_GAP;
    (pos, width, height.max(2.0 * MARGIN))
}

/// Середина сокета по вертикали: сначала выходы, потом входы (как у Blender).
fn socket_y(n: &Node, top: f32, output: bool, j: usize) -> f32 {
    let row = if output { j } else { n.outputs.len() + j };
    top + HEADER + 6.0 + row as f32 * ROW + ROW / 2.0
}

pub fn render(source: &str, cx: &Cx) -> Result<Element, String> {
    let g = parse(source);
    if g.nodes.is_empty() {
        return Err(cx.tr("нет нод: нода — строка без отступа, сокеты — «  > Вход: тип»", "no nodes: a node is an unindented line, sockets are “  > Input: type”").into());
    }
    let (pos, width, height) = layout(&g);
    let mut shapes = Vec::new();
    // Точечная сетка фона, как в редакторе нод.
    let mut y = 10.0;
    while y < height {
        let mut x = 10.0;
        while x < width {
            shapes.push(Shape::Rect { x, y, w: 1.5, h: 1.5, fill: Some("#2c2c2c".into()), stroke: None, stroke_width: 1.0, radius: Vec::new(), tooltip: String::new() });
            x += 20.0;
        }
        y += 20.0;
    }
    // Связи — под нодами: тёмная подложка и цветная середина от типа выхода к типу входа.
    let connected: Vec<(usize, usize)> = g.links.iter().map(|l| l.to).collect();
    for l in &g.links {
        let (a, b) = (&g.nodes[l.from.0], &g.nodes[l.to.0]);
        let p0 = [pos[l.from.0][0] + NODE_W, socket_y(a, pos[l.from.0][1], true, l.from.1)];
        let p3 = [pos[l.to.0][0], socket_y(b, pos[l.to.0][1], false, l.to.1)];
        let d = CURVING * (p3[0] - p0[0]).abs().max(40.0);
        let (c1, c2) = ([p0[0] + d, p0[1]], [p3[0] - d, p3[1]]);
        shapes.push(Shape::Bezier { from: p0, c1, c2, to: p3, color: "#1a1a1a".into(), color_to: None, width: 4.0 });
        shapes.push(Shape::Bezier { from: p0, c1, c2, to: p3, color: socket_color(&a.outputs[l.from.1].kind).into(), color_to: Some(socket_color(&b.inputs[l.to.1].kind).into()), width: 2.0 });
    }
    for (i, n) in g.nodes.iter().enumerate() {
        let [x, y] = pos[i];
        let h = node_height(n);
        // Тень, тело, шапка.
        shapes.push(Shape::Rect { x: x + 2.0, y: y + 3.0, w: NODE_W, h, fill: Some("#00000059".into()), stroke: None, stroke_width: 1.0, radius: vec![ROUND + 1.0], tooltip: String::new() });
        shapes.push(Shape::Rect { x, y, w: NODE_W, h, fill: Some("#303030".into()), stroke: Some("#000000aa".into()), stroke_width: 1.0, radius: vec![ROUND], tooltip: String::new() });
        shapes.push(Shape::Rect { x, y, w: NODE_W, h: HEADER, fill: Some(header_color(&n.class).into()), stroke: None, stroke_width: 1.0, radius: vec![ROUND, ROUND, 0.0, 0.0], tooltip: String::new() });
        shapes.push(Shape::Text { x: x + 10.0, y: y + HEADER / 2.0, text: fit(&n.name, NODE_W - 20.0, 12.5), size: 12.5, color: "#e6e6e6".into(), align: Align::Start, valign: Align::Center, bold: false, mono: false });
        for (j, s) in n.outputs.iter().enumerate() {
            let sy = socket_y(n, y, true, j);
            shapes.push(Shape::Text { x: x + NODE_W - 12.0, y: sy, text: fit(&s.name, NODE_W - 24.0, 12.0), size: 12.0, color: "#e6e6e6".into(), align: Align::End, valign: Align::Center, bold: false, mono: false });
            shapes.push(socket(x + NODE_W, sy, &s.kind, &s.name));
        }
        for (j, s) in n.inputs.iter().enumerate() {
            let sy = socket_y(n, y, false, j);
            let linked = connected.contains(&(i, j));
            // Свободный вход со значением — поле, как ползунок у Blender.
            if let (false, Some(v)) = (linked, &s.value) {
                shapes.push(Shape::Rect { x: x + 10.0, y: sy - 9.0, w: NODE_W - 20.0, h: 18.0, fill: Some("#545454".into()), stroke: None, stroke_width: 1.0, radius: vec![3.0], tooltip: String::new() });
                shapes.push(Shape::Text { x: x + NODE_W - 16.0, y: sy, text: fit(v, 52.0, 11.5), size: 11.5, color: "#e6e6e6".into(), align: Align::End, valign: Align::Center, bold: false, mono: false });
                shapes.push(Shape::Text { x: x + 16.0, y: sy, text: fit(&s.name, NODE_W - 80.0, 11.5), size: 11.5, color: "#e6e6e6".into(), align: Align::Start, valign: Align::Center, bold: false, mono: false });
            } else {
                shapes.push(Shape::Text { x: x + 12.0, y: sy, text: fit(&s.name, NODE_W - 24.0, 12.0), size: 12.0, color: "#e6e6e6".into(), align: Align::Start, valign: Align::Center, bold: false, mono: false });
            }
            shapes.push(socket(x, sy, &s.kind, &s.name));
        }
    }
    let canvas = Element::Canvas { width, height, shapes, pan_zoom: true, background: Some("#1d1d1d".into()) };
    let mut out = vec![canvas];
    for p in &g.problems {
        out.push(crate::util::warning(if cx.english { format!("Not understood: {p}") } else { format!("Не разобрано: {p}") }));
    }
    out.push(ui::hint(cx.tr("Тянуть — двигать, Ctrl и колесо — приблизить, двойной щелчок — как было.", "Drag to pan, Ctrl+wheel to zoom, double-click to reset.")));
    Ok(ui::column(out))
}

/// Сокет: кружок цвета типа с тёмной обводкой; подсказка — имя и тип.
fn socket(x: f32, y: f32, kind: &str, name: &str) -> Shape {
    Shape::Circle { x, y, r: SOCKET_R, fill: Some(socket_color(kind).into()), stroke: Some("#000000".into()), stroke_width: 1.0, tooltip: format!("{name}: {kind}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATERIAL: &str = "Image Texture [texture]\n  < Color: color\n  < Alpha: float\n  > Vector: vector\nPrincipled BSDF [shader]\n  < BSDF: shader\n  > Base Color: color\n  > Roughness: float = 0.5\nMaterial Output [output]\n  > Surface: shader\nImage Texture.Color -> Principled BSDF.Base Color\nPrincipled BSDF.BSDF -> Material Output.Surface\nNope.X -> Material Output.Surface\n";

    #[test]
    fn graph_reads_nodes_sockets_and_links() {
        let g = parse(MATERIAL);
        assert_eq!(g.nodes.len(), 3);
        assert_eq!((g.nodes[1].class.as_str(), g.nodes[1].outputs.len(), g.nodes[1].inputs.len()), ("shader", 1, 2));
        assert_eq!(g.nodes[1].inputs[1].value.as_deref(), Some("0.5"));
        assert_eq!(g.links, vec![Link { from: (0, 0), to: (1, 0) }, Link { from: (1, 0), to: (2, 0) }]);
        assert_eq!(g.problems, ["Nope.X -> Material Output.Surface"]);
        assert_eq!(columns(&g), [0, 1, 2], "столбцы по связям");
    }

    #[test]
    fn nodes_are_laid_out_left_to_right_and_drawn() {
        let g = parse(MATERIAL);
        let (pos, width, height) = layout(&g);
        assert!(pos[0][0] < pos[1][0] && pos[1][0] < pos[2][0]);
        assert_eq!(width, 2.0 * MARGIN + 3.0 * NODE_W + 2.0 * COL_GAP);
        assert!(height >= node_height(&g.nodes[1]) + 2.0 * MARGIN);
        let json = render(MATERIAL, &Cx::default()).unwrap().to_json();
        assert!(json.contains("#2b652b") && json.contains("#c7c729") && json.contains(r#""type":"bezier""#), "шапка shader, сокет color, связи");
        assert!(json.contains("Не разобрано: Nope.X"));
    }

    #[test]
    fn cycles_do_not_hang_the_layout() {
        let g = parse("A\n  < o: float\n  > i: float\nB\n  < o: float\n  > i: float\nA.o -> B.i\nB.o -> A.i\n");
        assert_eq!(g.links.len(), 2);
        let _ = layout(&g);
    }
}
