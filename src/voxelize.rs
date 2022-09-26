use std::collections::HashMap;

use crate::barycentric::interpolate_uv;
use crate::color::*;
use crate::intersect::intersect;
use crate::load::{MeshExt, ModelSet, ObjMaterial, ObjRegistry};
use crate::octree::{Branches, TreeBody, VoxelTree};
use crate::BrickType;

use image::RgbaImage;
use nalgebra::{Matrix4, Point3, Projective3, Vector2, Vector3, Vector4};
use parry3d::bounding_volume::AABB;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct Triangle {
    material_id: Option<usize>,
    vertices: [Vector3<f32>; 3],
    uvs: Option<[Vector2<f32>; 3]>,
}

impl ModelSet {
    pub fn voxelize(
        &self,
        images: &HashMap<String, image::RgbaImage>,
        transform: &Projective3<f32>,
    ) -> VoxelTree<Vector4<u8>> {
        let mut octree = VoxelTree::<Vector4<u8>>::new();

        let aabb = AABB::new_invalid();
        for model in &self.models {
            let mesh = &model.mesh;
            let material = mesh.material_id.map(|id| &self.materials[id]);
            // the material is applied to every triangle in the mesh, so there's no reason to
            // bother with it if it's invisible
            if let Some(ObjMaterial::Color(_, _, _, 0.0)) = material {
                tracing::debug!(
                    "Skipping mesh with invisible material: {}.{}",
                    &model.name,
                    mesh.material_id.as_ref().unwrap()
                );
                continue;
            }
            // prepare & transform vertices
            let verts = vec![];
            for v in mesh.vertices() {
                let pos = &mut v[0..3];
                let p = transform.transform_point(&Point3::from_slice(pos));
                pos[0] = p.x;
                pos[1] = p.y;
                pos[2] = p.z;
                // expand aabb
                aabb.take_point(p);
                verts.push(v);
            }
            // voxelize each triangle
            for tri in mesh.triangles() {
                let a = verts[tri[0] as usize];
                let b = verts[tri[1] as usize];
                let c = verts[tri[2] as usize];
            }
        }

        let floor_min = Vector3::<isize>::new(
            min[0].floor() as isize - 1,
            min[1].floor() as isize - 1,
            min[2].floor() as isize - 1,
        );
        let ceil_max = Vector3::<isize>::new(
            max[0].ceil() as isize + 1,
            max[1].ceil() as isize + 1,
            max[2].ceil() as isize + 1,
        );

        while !octree.contains_bounds(floor_min) || !octree.contains_bounds(ceil_max) {
            octree.size += 1;
        }

        let mask = 1 << octree.size; // mask = 1ᵗ, where `t` is the size of the octree

        recursive_voxelize(
            &mut octree.contents,
            mask,
            triangles,
            images,
            &self.materials,
        );

        octree
    }
}

impl ObjRegistry {
    pub fn voxelize(&self, scale: f32, bricktype: BrickType) -> VoxelTree<Vector4<u8>> {
        todo!()
    }
}

fn recursive_voxelize<'a>(
    branches: &'a mut Branches<Vector4<u8>>,
    mask: isize,
    vector: Vec<Triangle>,
    images: &HashMap<String, RgbaImage>,
    materials: &[ObjMaterial],
) {
    let m = mask >> 1;
    let half_box = (2 * m + ((m == 0) as isize)) as f32 / 2.;

    for (i, branch) in branches.iter_mut().enumerate() {
        if let TreeBody::Empty = branch {
            let center = Vector3::<f32>::new(
                half_box * (2 * ((i & 4) > 0) as isize - 1) as f32,
                half_box * (2 * ((i & 2) > 0) as isize - 1) as f32,
                half_box * (2 * ((i & 1) > 0) as isize - 1) as f32,
            );

            let mut triangles = Vec::<Triangle>::new();
            let mut colors = Vec::<Vector4<u8>>::new();

            for triangle in &vector {
                match intersect(
                    half_box,
                    center,
                    triangle.vertices[0],
                    triangle.vertices[1],
                    triangle.vertices[2],
                ) {
                    Some(intersection) => {
                        // Only calculate colors if in root level
                        if m == 0 {
                            if let Some(id) = triangle.material_id {
                                let uv =
                                    interpolate_uv(&triangle.vertices, &triangle.uvs, intersection);
                                let mat = &materials[id];

                                let c = match mat {
                                    ObjMaterial::Color(r, g, b, a) => [
                                        (255.0 * r) as u8,
                                        (255.0 * g) as u8,
                                        (255.0 * b) as u8,
                                        (255.0 * a) as u8,
                                    ],
                                    ObjMaterial::ImageKey(img) => {
                                        let img = images[img];
                                        let u = ((uv[0] - uv[0].floor()) * (img.width() - 1) as f32)
                                            as u32;
                                        let v = ((1. - uv[1] + uv[1].floor())
                                            * (img.height() - 1) as f32)
                                            as u32;
                                        img.get_pixel(u, v).0
                                    }
                                };

                                if c[3] == 0 {
                                    continue;
                                } // If alpha is zero, skeedaddle
                                colors.push(Vector4::<u8>::new(c[0], c[1], c[2], c[3]));
                            }
                        }
                    }
                    None => continue,
                }

                let mut cloned_triangle = *triangle;
                cloned_triangle.vertices[0] -= center;
                cloned_triangle.vertices[1] -= center;
                cloned_triangle.vertices[2] -= center;

                triangles.push(cloned_triangle);
            }

            if triangles.is_empty() {
                continue;
            }
            if m != 0 {
                // Not yet at root level, keep on recursing...
                *branch = TreeBody::Branch(Box::new(TreeBody::empty()));
                if let TreeBody::Branch(b) = branch {
                    recursive_voxelize(b, m, triangles, images, materials);
                }
            } else {
                *branch = TreeBody::Leaf(hsv2rgb(hsv_average(&colors)));
            }
        }
    }
}
