use std::{fmt::Debug, ops::{Add, Deref, DerefMut, Index, IndexMut, Range}, collections::HashMap};

pub struct VertexDescriptor {
    length: usize,
    fields: HashMap<String, Range<usize>>
}

pub struct Mesh<'desc, Data, Index = usize> {
    vert_data: Vec<Data>,
    vert_descriptor: &'desc VertexDescriptor,
    face_indices: Vec<Index>,
    face_arities: Vec<Index>,
}

impl<D: Debug, I: Debug> Debug for Mesh<'_, D, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mesh").field("vert_data", &self.vert_data).field("vert_descriptor", &self.vert_descriptor).field("face_indices", &self.face_indices).field("face_arities", &self.face_arities).finish()
    }
}

impl<D: Clone, I: Clone> Clone for Mesh<'_, D, I> {
    fn clone(&self) -> Self {
        Self {
            vert_data: self.vert_data.clone(),
            vert_descriptor: self.vert_descriptor,
            face_indices: self.face_indices.clone(),
            face_arities: self.face_arities.clone()
        }
    }
}

impl<'desc, D, I: Into<usize> + Copy> Mesh<'desc, D, I> {
    pub fn new(descriptor: &'desc VertexDescriptor) -> Self {
        Self {
            vert_data: Default::default(),
            vert_descriptor: descriptor,
            face_indices: Default::default(),
            face_arities: Default::default()
        }
    }
    pub fn vertex(&self, index: usize) -> &[D] { &self.vert_data[index..index+self.vert_descriptor.length] }
    pub fn vertex_mut(&mut self, index: usize) -> &mut [D] { &mut self.vert_data[index..index+self.vert_descriptor.length] }
    pub fn vertices(&self) -> impl Iterator<Item = &[D]> { self.vert_data.chunks_exact(self.vert_descriptor.length) }
    pub fn vertices_mut(&mut self) -> impl Iterator<Item = &mut [D]> { self.vert_data.chunks_exact_mut(self.vert_descriptor.length) }
    pub fn face_indices(&self) -> Box<dyn Iterator<Item = &[I]>> where I: Into<usize> + Copy {
        match self.face_arities.len() {
            0 => Box::new(self.face_indices.chunks_exact(3)),
            _ => Box::new(self.face_arities.iter().scan(0usize, |i, a| {
                let start = *i;
                *i = *i + (*a).into();
                Some(&self.face_indices[start..*i])
            }))
        }
    }
    pub fn faces<'h>(&'h self) -> impl Iterator<Item = Polygon<'h, D, I>> where I: Into<usize> + Copy {
        self.face_indices().map(|face| Polygon { mesh: self, verts: face.iter().map(|fi| self.vertex((*fi).into())).collect() })
    }
    pub fn triangles<'h>(&'h self) -> impl Iterator<Item = Triangle<'h, D, I>> where I: Into<usize> + Copy {
        self.face_indices().flat_map(|face| {
            match face {
                [a] => [],
                [a, b] => [],
                [a, b, c] => [Triangle::new(self, self.vertex((*a).into()), self.vertex((*b).into()), self.vertex((*c).into()))],
                [a, b, c, d] => [Triangle::new(self, self.vertex((*a).into()), self.vertex((*b).into()), self.vertex((*c).into()))],
                _ => todo!()
            }
        })
    }
}

pub struct Polygon<'mesh, Data, MIdx> {
    mesh: &'mesh Mesh<'mesh, Data, MIdx>,
    verts: Vec<&'mesh [Data]>,
}

impl<'mesh, Data, MIdx> Index<usize> for Polygon<'mesh, Data, MIdx> {
    type Output = [Data];
    fn index(&self, index: usize) -> &'mesh Self::Output {
        self.vertex(index)
    }
}

impl<'mesh, Data, MIdx> Polygon<'mesh, Data, MIdx> {
    pub fn vertices(&self) -> &[&'mesh [Data]] { &self.verts }
    pub fn vertex(&self, index: usize) -> &'mesh [Data] { self.verts[index] }
}

pub struct Triangle<'mesh, Data, MIdx> {
    mesh: &'mesh Mesh<'mesh, Data, MIdx>,
    verts: [&'mesh [Data]; 3]
}

impl<'mesh, Data, MIdx> Triangle<'mesh, Data, MIdx> {
    fn new(mesh: &'mesh Mesh<'mesh, Data, MIdx>, a: &'mesh [Data], b: &'mesh [Data], c: &'mesh [Data]) -> Self {
        Self {
            mesh,
            verts: [a, b, c]
        }
    }

    fn from_quad_indices(mesh: &'mesh Mesh<'mesh, Data, MIdx>, [a, b, c, d]: &[MIdx]) -> (Self, Self) where MIdx: Into<usize> + Copy {
        (Self)
    }
}
