//! Генерация helix-меша пружины (03-scales §3.5).
//!
//! Меш строится один раз в единичной высоте; дальше меняется только
//! `Transform.scale.y` (известный артефакт — толщина проволоки
//! масштабируется вместе с высотой — допустим в MVP, исправление на T07).

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;

/// Параметры пружины (03-scales §3.5).
#[derive(Clone, Copy, Debug)]
pub struct HelixParams {
    pub turns: f32,
    pub radius: f32,
    pub wire_radius: f32,
    pub samples_per_turn: usize,
    pub height: f32,
    /// Количество вершин в сечении проволоки.
    pub radial_segments: usize,
}

impl Default for HelixParams {
    fn default() -> Self {
        Self {
            turns: 6.0,
            radius: 0.09,
            wire_radius: 0.012,
            samples_per_turn: 24,
            height: 1.0,
            radial_segments: 8,
        }
    }
}

/// Осевая линия спирали: `center(u) = (r·cos θ, height·u, r·sin θ)`.
fn helix_center(p: &HelixParams, u: f32) -> [f32; 3] {
    let theta = std::f32::consts::TAU * p.turns * u;
    [
        p.radius * theta.cos(),
        p.height * u,
        p.radius * theta.sin(),
    ]
}

/// Считает вершины трубки: для каждого кольца — окружность радиуса
/// `wire_radius` в плоскости, перпендикулярной касательной.
fn tube_vertices(p: &HelixParams) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let n_rings = p.turns as usize * p.samples_per_turn + 1;
    let seg = p.radial_segments;
    let mut positions = Vec::with_capacity(n_rings * seg);
    let mut normals = Vec::with_capacity(n_rings * seg);

    for i in 0..n_rings {
        let u = i as f32 / (n_rings - 1) as f32;
        let c = helix_center(p, u);
        // Касательная — конечной разностью (на концах односторонняя).
        let du = (1.0 / (n_rings - 1) as f32).min(0.5);
        let a = helix_center(p, (u - du).max(0.0));
        let b = helix_center(p, (u + du).min(1.0));
        let tangent = norm3([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
        // Ортонормированный базис плоскости сечения.
        let n1 = norm3(cross(tangent, [0.0, 1.0, 0.0]));
        let n2 = norm3(cross(tangent, n1));

        for s in 0..seg {
            let phi = std::f32::consts::TAU * s as f32 / seg as f32;
            let dir = [
                phi.cos() * n1[0] + phi.sin() * n2[0],
                phi.cos() * n1[1] + phi.sin() * n2[1],
                phi.cos() * n1[2] + phi.sin() * n2[2],
            ];
            positions.push([
                c[0] + p.wire_radius * dir[0],
                c[1] + p.wire_radius * dir[1],
                c[2] + p.wire_radius * dir[2],
            ]);
            normals.push(dir);
        }
    }
    (positions, normals)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-8 {
        [1.0, 0.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

/// Собирает `Mesh` пружины: TriangleList, нормали радиальные.
pub fn helix_mesh(p: HelixParams) -> Mesh {
    let (positions, normals) = tube_vertices(&p);
    let seg = p.radial_segments;
    let n_rings = positions.len() / seg;
    let mut indices = Vec::with_capacity((n_rings - 1) * seg * 6);

    for i in 0..n_rings - 1 {
        for s in 0..seg {
            let s1 = (s + 1) % seg;
            let a = (i * seg + s) as u32;
            let b = (i * seg + s1) as u32;
            let c = ((i + 1) * seg + s) as u32;
            let d = ((i + 1) * seg + s1) as u32;
            // Наружная обмотка: нормаль треугольника смотрит от осевой
            // линии трубки (проверяется тестом winding_is_outward).
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Размеры: 6 витков × 24 выборки = 145 колец × 8 вершин.
    #[test]
    fn vertex_and_index_counts() {
        let p = HelixParams::default();
        let mesh = helix_mesh(p);
        let n_vertices = (p.turns as usize * p.samples_per_turn + 1) * p.radial_segments;
        let pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(pos.as_float3().unwrap().len(), n_vertices);
        let idx = mesh.indices().unwrap();
        assert_eq!(idx.len(), (n_vertices / p.radial_segments - 1) * p.radial_segments * 6);
    }

    /// Трубка лежит в границах [0, height] по Y и не вылезает за радиус.
    #[test]
    fn geometry_bounds() {
        let p = HelixParams::default();
        let mesh = helix_mesh(p);
        let pos = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let mut max_r = 0.0_f32;
        // Сечение проволоки перпендикулярно касательной, поэтому трубка
        // выступает за [0, height] не более чем на wire_radius.
        for v in pos.as_float3().unwrap() {
            assert!(
                ((-p.wire_radius - 1e-5)..=(p.height + p.wire_radius + 1e-5)).contains(&v[1]),
                "y = {}",
                v[1]
            );
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            max_r = max_r.max(r);
        }
        assert!(max_r <= p.radius + p.wire_radius + 1e-5);
    }

    /// Обмотка треугольников — наружная (нормаль от осевой линии трубки).
    #[test]
    fn winding_is_outward() {
        let p = HelixParams::default();
        let mesh = helix_mesh(p);
        let pos = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap()
            .to_vec();
        let first: Vec<usize> = mesh.indices().unwrap().iter().take(3).collect();
        let (a, b, c) = (pos[first[0]], pos[first[1]], pos[first[2]]);
        let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = cross(e1, e2);
        let center = helix_center(&p, 0.0);
        // Радиаль первого кольца — от осевой линии к вершине `a`.
        let radial = [a[0] - center[0], a[1] - center[1], a[2] - center[2]];
        assert!(dot3(n, radial) > 0.0, "нормаль внутрь трубки");
    }

    fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }
}
