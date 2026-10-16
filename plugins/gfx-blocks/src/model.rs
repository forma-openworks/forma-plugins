//! Модель glTF 2.0 (`.glb` или `.gltf`) из хранилища: все треугольники сцены в мировых
//! координатах (с трансформами узлов), UV, сведения для курса — сколько треугольников,
//! вершин, материалов, есть ли UV и нормали, размеры.
//!
//! Файлы читает программа (`read-bytes`): буферы `.glb` лежат внутри, у `.gltf` — рядом
//! или прямо в нём (`data:`-адрес).

use base64::Engine;

/// Больше треугольников в просмотр не отдаём: остальное прореживается (сведения — по всей
/// модели).
pub const MAX_TRIS: usize = 150_000;

#[derive(Debug, Default, Clone)]
pub struct Model {
    /// Вершины подряд (`x, y, z`) в мировых координатах.
    pub positions: Vec<f32>,
    /// Треугольники: номера вершин по три.
    pub indices: Vec<u32>,
    /// UV каждой вершины (`u, v`); у вершин без UV — нули.
    pub uvs: Vec<f32>,
    /// Из какого меша каждый треугольник (номер в `mesh_names`).
    pub tri_mesh: Vec<u32>,
    pub mesh_names: Vec<String>,
    pub primitives: usize,
    pub materials: usize,
    /// Есть ли UV и нормали у всех частей.
    pub has_uv: bool,
    pub has_normals: bool,
    /// Части не из треугольников (линии, точки) — пропущены.
    pub skipped: usize,
}

type Mat4 = [[f32; 4]; 4];

const IDENTITY: Mat4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// Произведение матриц glTF (по столбцам): `a · b`.
fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0f32; 4]; 4];
    for (c, col) in out.iter_mut().enumerate() {
        for (r, cell) in col.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    out
}

fn apply(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
    let v = |r: usize| m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r];
    [v(0), v(1), v(2)]
}

/// Папка файла (для буферов `.gltf` рядом с ним).
fn dir_of(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..=i])
}

/// Загружает модель `path`; `read` читает файлы хранилища.
pub fn load(path: &str, read: &dyn Fn(&str) -> Result<Vec<u8>, String>) -> Result<Model, String> {
    let bytes = read(path)?;
    let gltf = gltf::Gltf::from_slice(&bytes).map_err(|e| format!("{path}: {e}"))?;
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    for b in gltf.buffers() {
        let data = match b.source() {
            gltf::buffer::Source::Bin => gltf.blob.clone().ok_or("the .glb has no binary chunk")?,
            gltf::buffer::Source::Uri(uri) => match uri.strip_prefix("data:") {
                Some(data) => {
                    let b64 = data.split_once(',').map_or("", |(_, d)| d);
                    base64::engine::general_purpose::STANDARD.decode(b64).map_err(|e| format!("data URI: {e}"))?
                }
                None => read(&format!("{}{}", dir_of(path), uri.replace("%20", " ")))?,
            },
        };
        buffers.push(data);
    }
    let mut m = Model { has_uv: true, has_normals: true, materials: gltf.materials().count(), ..Default::default() };
    m.mesh_names = gltf.meshes().map(|mesh| mesh.name().map_or_else(|| format!("Mesh {}", mesh.index()), str::to_owned)).collect();
    let scene = gltf.default_scene().or_else(|| gltf.scenes().next());
    match scene {
        Some(scene) => {
            for node in scene.nodes() {
                walk(&node, &IDENTITY, &buffers, &mut m);
            }
        }
        // Без сцены — меши как есть.
        None => {
            for mesh in gltf.meshes() {
                add_mesh(&mesh, &IDENTITY, &buffers, &mut m);
            }
        }
    }
    if m.indices.is_empty() {
        return Err(format!("{path}: no triangles"));
    }
    Ok(m)
}

fn walk(node: &gltf::Node, parent: &Mat4, buffers: &[Vec<u8>], m: &mut Model) {
    let world = mul(parent, &node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        add_mesh(&mesh, &world, buffers, m);
    }
    for child in node.children() {
        walk(&child, &world, buffers, m);
    }
}

fn add_mesh(mesh: &gltf::Mesh, world: &Mat4, buffers: &[Vec<u8>], m: &mut Model) {
    for prim in mesh.primitives() {
        if prim.mode() != gltf::mesh::Mode::Triangles {
            m.skipped += 1;
            continue;
        }
        let reader = prim.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
        let Some(positions) = reader.read_positions() else {
            m.skipped += 1;
            continue;
        };
        let base = (m.positions.len() / 3) as u32;
        let mut count = 0u32;
        for p in positions {
            m.positions.extend(apply(world, p));
            count += 1;
        }
        match reader.read_tex_coords(0) {
            Some(uv) => m.uvs.extend(uv.into_f32().take(count as usize).flatten()),
            None => m.has_uv = false,
        }
        // UV на каждую вершину, даже если их меньше (битый файл) или нет совсем.
        m.uvs.resize(m.positions.len() / 3 * 2, 0.0);
        m.has_normals &= reader.read_normals().is_some();
        let indices: Vec<u32> = match reader.read_indices() {
            Some(i) => i.into_u32().collect(),
            None => (0..count).collect(),
        };
        for tri in indices.as_chunks::<3>().0 {
            if tri.iter().all(|&i| i < count) {
                m.indices.extend(tri.map(|i| base + i));
                m.tri_mesh.push(mesh.index() as u32);
            }
        }
        m.primitives += 1;
    }
}

impl Model {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertices(&self) -> usize {
        self.positions.len() / 3
    }

    /// Размеры по осям (как Dimensions в Blender, в единицах файла — обычно метрах).
    pub fn size(&self) -> [f32; 3] {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in self.positions.as_chunks::<3>().0 {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        if lo[0] > hi[0] { [0.0; 3] } else { [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]] }
    }

    /// Треугольники для просмотра: не больше [`MAX_TRIS`], равномерно по модели.
    pub fn preview_indices(&self) -> Vec<u32> {
        let tris = self.triangles();
        if tris <= MAX_TRIS {
            return self.indices.clone();
        }
        let step = tris.div_ceil(MAX_TRIS);
        self.indices.as_chunks::<3>().0.iter().step_by(step).flatten().copied().collect()
    }
}

/// Читает файлы хранилища через программу.
pub fn read_vault(path: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_arch = "wasm32")]
    return forma_plugin_api::host::read_bytes(path);
    #[cfg(not(target_arch = "wasm32"))]
    return Err(format!("{path}: no host outside the app"));
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Маленький `.gltf` с буфером в `data:`: два треугольника (квадрат 2×1) с UV, узел
    /// сдвинут на +10 по X.
    pub fn quad_gltf() -> String {
        let mut bin = Vec::new();
        for p in [[0.0f32, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 1.0, 0.0], [0.0, 1.0, 0.0]] {
            for c in p {
                bin.extend(c.to_le_bytes());
            }
        }
        for uv in [[0.0f32, 1.0], [0.5, 1.0], [0.5, 0.5], [0.0, 0.5]] {
            for c in uv {
                bin.extend(c.to_le_bytes());
            }
        }
        for i in [0u16, 1, 2, 0, 2, 3] {
            bin.extend(i.to_le_bytes());
        }
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bin);
        format!(
            r#"{{"asset":{{"version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],
            "nodes":[{{"mesh":0,"translation":[10,0,0]}}],
            "meshes":[{{"name":"Wing","primitives":[{{"attributes":{{"POSITION":0,"TEXCOORD_0":1}},"indices":2}}]}}],
            "materials":[{{"name":"Camo"}}],
            "buffers":[{{"byteLength":{},"uri":"data:application/octet-stream;base64,{b64}"}}],
            "bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":48}},{{"buffer":0,"byteOffset":48,"byteLength":32}},{{"buffer":0,"byteOffset":80,"byteLength":12}}],
            "accessors":[
              {{"bufferView":0,"componentType":5126,"count":4,"type":"VEC3","min":[0,0,0],"max":[2,1,0]}},
              {{"bufferView":1,"componentType":5126,"count":4,"type":"VEC2"}},
              {{"bufferView":2,"componentType":5123,"count":6,"type":"SCALAR"}}]}}"#,
            bin.len()
        )
    }

    #[test]
    fn a_gltf_with_a_data_buffer_loads_in_world_space() {
        let file = quad_gltf();
        let read = move |p: &str| if p == "m/quad.gltf" { Ok(file.clone().into_bytes()) } else { Err(format!("{p}: missing")) };
        let m = load("m/quad.gltf", &read).unwrap();
        assert_eq!((m.triangles(), m.vertices(), m.materials, m.primitives), (2, 4, 1, 1));
        assert!(m.has_uv && !m.has_normals);
        assert_eq!(&m.positions[..3], &[10.0, 0.0, 0.0], "трансформ узла применён");
        assert_eq!(m.size(), [2.0, 1.0, 0.0]);
        assert_eq!(m.mesh_names, ["Wing"]);
        assert_eq!(m.uvs.len(), 8);
        assert!(load("m/none.glb", &read).is_err());
    }

    #[test]
    fn matrices_compose_parent_then_child() {
        let t = |x: f32| [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [x, 0.0, 0.0, 1.0]];
        let s2: Mat4 = [[2.0, 0.0, 0.0, 0.0], [0.0, 2.0, 0.0, 0.0], [0.0, 0.0, 2.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        // Родитель сдвигает на 5, ребёнок масштабирует вдвое: точка 1 → 2 → 7.
        assert_eq!(apply(&mul(&t(5.0), &s2), [1.0, 0.0, 0.0]), [7.0, 0.0, 0.0]);
    }
}

#[cfg(test)]
mod real {
    /// Настоящая модель с диска: `GFX_TEST_GLB=<путь к .glb>`.
    #[test]
    fn real_glb_has_sane_triangles() {
        let Some(path) = std::env::var_os("GFX_TEST_GLB") else { return };
        let path = path.to_string_lossy().into_owned();
        let read = |p: &str| std::fs::read(p).map_err(|e| e.to_string());
        let m = super::load(&path, &read).unwrap();
        let n = m.vertices() as u32;
        assert!(m.indices.iter().all(|&i| i < n));
        let size = m.size();
        eprintln!("tris {} verts {} size {size:?} prims {} first {:?}", m.triangles(), m.vertices(), m.primitives, &m.positions[..9]);
        let max_edge = m.indices.as_chunks::<3>().0.iter().map(|t| {
            let p = |i: u32| [m.positions[i as usize * 3], m.positions[i as usize * 3 + 1], m.positions[i as usize * 3 + 2]];
            let d = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            d(p(t[0]), p(t[1])).max(d(p(t[1]), p(t[2])))
        }).fold(0.0f32, f32::max);
        assert!(max_edge < size.iter().copied().fold(0.0, f32::max), "рёбра не длиннее самой модели: {max_edge}");
    }
}
