use std::{
    convert::TryInto,
    fmt::Debug,
    marker::PhantomData,
    ops::Range,
    slice::{Chunks, ChunksExact, ChunksExactMut, ChunksMut},
};

use enumflags2::BitFlags;
use nalgebra::{
    Const, Matrix, MatrixSlice, MatrixSlice3x1, MatrixSliceMut, MatrixSliceMut3x1, Point3,
    Projective3, SliceStorage, SliceStorageMut,
};
use parry3d::{bounding_volume::AABB, math::Point as PPoint};
use tobj::Mesh;

#[derive(Default, Debug, Clone)]
pub struct ObjFaceRef<'mesh> {
    positions: Vec<&'mesh [f32; 3]>,
    normals: Vec<&'mesh [f32; 3]>,
    colors: Vec<&'mesh [f32; 3]>,
    uvs: Vec<&'mesh [f32; 2]>,
}

impl<'mesh> ObjFaceRef<'mesh> {
    fn new(mesh: &'mesh Mesh, verts: impl IntoIterator<Item = u32>) -> Self {
        let mut positions = vec![];
        let mut normals = vec![];
        let mut colors = vec![];
        let mut uvs = vec![];
        for v in verts {
            let v = v as usize;

            let pi = mesh.indices[v] as usize;
            let ci = mesh.vertex_color_indices.get(v).map_or(pi, |c| *c as usize);
            let ni = mesh.normal_indices.get(v).map_or(pi, |n| *n as usize);
            let ti = mesh.texcoord_indices.get(v).map_or(pi, |t| *t as usize);

            positions.push(&mesh.positions[pi..pi + 3].try_into().unwrap());
            if let Some(n) = mesh.normals.get(ni..ni + 3) {
                normals.push(n.try_into().unwrap());
            }
            if let Some(c) = mesh.vertex_color.get(ci..ci + 3) {
                colors.push(c.try_into().unwrap());
            }
            if let Some(t) = mesh.texcoords.get(ti..ti + 2) {
                uvs.push(t.try_into().unwrap());
            }
        }
        ObjFaceRef {
            positions,
            normals,
            colors,
            uvs,
        }
    }
}

#[enumflags2::bitflags]
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum VertFieldFlags {
    Normal = 0b0001,
    Color = 0b0010,
    Uv = 0b0100,
}

#[derive(Debug, Default, Clone)]
pub struct ObjFace {
    vert_data: Vec<f32>,
    vert_fields: BitFlags<VertFieldFlags>,
    vert_len: usize,
}

impl ObjFace {
    fn vertices(&self) -> impl Iterator<Item = &[f32]> {
        self.vert_data.chunks_exact(self.vert_len)
    }
    fn vertices_mut(&mut self) -> impl Iterator<Item = &mut [f32]> {
        self.vert_data.chunks_exact_mut(self.vert_len)
    }
}

impl<'mesh> From<ObjFaceRef<'mesh>> for ObjFace {
    fn from(o: ObjFaceRef<'mesh>) -> Self {
        use VertFieldFlags as Field;
        let mut fields = BitFlags::<VertFieldFlags>::empty();
        let vert_data = vec![];
        for v in 0..o.positions.len() {
            vert_data.extend_from_slice(o.positions[v]);
            if let Some(n) = o.normals.get(v) {
                vert_data.extend_from_slice(*n);
                fields |= Field::Normal;
            }
            if let Some(c) = o.colors.get(v) {
                vert_data.extend_from_slice(*c);
                fields |= Field::Color;
            }
            if let Some(t) = o.uvs.get(v) {
                vert_data.extend_from_slice(*t);
                fields |= Field::Uv;
            }
        }
        let bits = fields.bits();
        Self {
            vert_data,
            vert_fields: fields,
            vert_len: (3
                + (((fields & Field::Normal).bits() >> 0) * 3)
                + (((fields & Field::Color).bits() >> 1) * 3)
                + (((fields & Field::Uv).bits() >> 2) * 2)) as usize,
        }
    }
}

// #[derive(Default, Debug)]
// pub struct ObjFaceMut<'mesh> {
//     vertices: Vec<ObjVertexMut<'mesh>>,
//     _lifetime: PhantomData<&'mesh mut ()>,
// }

#[derive(Debug, Clone, Copy)]
pub struct ObjVertexRef<'mesh> {
    position: &'mesh [f32; 3],
    normal: Option<&'mesh [f32; 3]>,
    color: Option<&'mesh [f32; 3]>,
    uv: Option<&'mesh [f32; 2]>,
}

// #[derive(Debug)]
// pub struct ObjVertexMut<'mesh> {
//     position: &'mesh mut [f32; 3],
//     color: Option<&'mesh mut [f32; 3]>,
//     normal: Option<&'mesh mut [f32; 3]>,
//     uv: Option<&'mesh mut [f32; 2]>,
// }

pub trait MeshExt {
    fn raw_points(&self) -> ChunksExact<f32>;
    fn raw_points_mut(&mut self) -> ChunksExactMut<f32>;

    fn vertices(&self) -> Box<dyn Iterator<Item = Vec<f32>>>;

    fn vertex_format(&self) -> BitFlags<VertFieldFlags>;

    fn aabb(&self) -> AABB {
        let mut res = AABB::new_invalid();
        for [x, y, z] in self.raw_points() {
            res.take_point(PPoint::new(*x, *y, *z));
        }
        res
    }

    fn points<'mesh>(&'mesh self) -> Box<dyn Iterator<Item = (&[f32; 3], Point3<f32>)>> {
        Box::new(
            self.raw_points()
                .map(|p| (p.try_into().unwrap(), Point3::from_slice(p))),
        )
    }
    fn points_mut<'mesh>(
        &'mesh mut self,
    ) -> Box<dyn Iterator<Item = (&'mesh mut [f32; 3], Point3<f32>)>> {
        Box::new(
            self.raw_points_mut()
                .map(|p| (p.try_into().unwrap(), Point3::from_slice(p))),
        )
    }
    fn faces<'mesh>(&'mesh self) -> Box<dyn Iterator<Item = ObjFaceRef<'mesh>> + '_>;
    fn triangles<'mesh>(&'mesh self) -> Box<dyn Iterator<Item = [u32; 3]> + '_>;
}

impl MeshExt for tobj::Mesh {
    fn raw_points(&self) -> ChunksExact<f32> {
        self.positions.chunks_exact(3)
    }

    fn raw_points_mut(&mut self) -> ChunksExactMut<f32> {
        self.positions.chunks_exact_mut(3)
    }

    fn vertex_format(&self) -> BitFlags<VertFieldFlags> {
        use VertFieldFlags as Field;
        unsafe {
            BitFlags::<VertFieldFlags>::from_bits_unchecked(
                0b0000
                    | (Field::Normal as u8 * (!self.normals.is_empty() as u8))
                    | (Field::Color as u8 * (!self.vertex_color.is_empty() as u8))
                    | (Field::Uv as u8 * (!self.texcoords.is_empty() as u8)),
            )
        }
    }

    fn vertices(&self) -> Box<dyn Iterator<Item = Vec<f32>>> {
        Box::new(self.raw_points().enumerate().map(|(v, [x, y, z])| {
            let mut res = vec![*x, *y, *z];
            let v3 = v * 3;
            let v2 = v * 2;
            if let Some(n) = self.normals.get(v3..v3 + 3) {
                res.extend_from_slice(n);
            }
            if let Some(c) = self.vertex_color.get(v3..v3 + 3) {
                res.extend_from_slice(c);
            }
            if let Some(t) = self.texcoords.get(v2..v2 + 2) {
                res.extend_from_slice(t);
            }
            res
        }))
    }

    fn triangles(&self) -> Box<dyn Iterator<Item = [u32; 3]> + '_> {
        match self.face_arities.len() {
            0 => Box::new(self.indices.chunks_exact(3).map(|c| c.try_into().unwrap()))
                as Box<dyn Iterator<Item = [u32; 3]> + '_>, // they're already triangles
            _ => Box::new(
                self.face_arities
                    .iter()
                    .scan(0u32, |i, a| {
                        let start = *i;
                        *i = *i + *a;
                        match a {
                            0 => unreachable!(),
                            1 => Some(vec![]), // skip points
                            2 => Some(vec![]), // skip lines
                            3 => Some(vec![[start, start + 1, start + 2]]),
                            4 => Some(vec![
                                [start, start + 1, start + 2],
                                [start, start + 2, start + 3],
                            ]),
                            x => Some({
                                let res = vec![];
                                for c in start + 2..*i {
                                    res.push([start, c - 1, c]);
                                }
                                res
                            }),
                        }
                    })
                    .flatten(),
            ),
        }
    }

    fn faces<'mesh>(&'mesh self) -> Box<dyn Iterator<Item = ObjFaceRef<'mesh>> + '_> {
        match self.face_arities.len() {
            0 => Box::new(self.indices.chunks_exact(3).scan(0u32, |i, c| {
                let start = *i;
                *i = *i + 3;
                Some(ObjFaceRef::new(self, start..*i))
            })),

            _ => Box::new(self.face_arities.iter().scan(0u32, |i, a| {
                let start = *i;
                *i = *i + *a;
                Some(ObjFaceRef::new(self, start..*i))
            })),
        }
    }
}
