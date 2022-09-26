use std::{
    cell::RefCell,
    collections::HashMap,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use nalgebra::{Affine3, Point3, Projective3, Scale3, Transform3, Vector3};

use crate::BrickType;

mod obj_ext;
pub use obj_ext::*;

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Model(#[from] tobj::LoadError),
    #[error("Failed to load {0} texture file from {1:?}: {2}")]
    Image(String, PathBuf, image::ImageError),
}

#[derive(Debug)]
pub enum ObjMaterial {
    Color(f32, f32, f32, f32),
    ImageKey(String),
}

#[derive(Debug, Default)]
pub struct ModelSet {
    pub models: Vec<tobj::Model>,
    pub materials: Vec<ObjMaterial>,
}

impl ModelSet {
    pub fn raw_points(&self) -> impl Iterator<Item = &[f32]> + '_ {
        self.models.iter().flat_map(|m| m.mesh.raw_points())
    }

    pub fn raw_points_mut(&mut self) -> impl Iterator<Item = &mut [f32]> + '_ {
        self.models.iter_mut().flat_map(|m| m.mesh.raw_points_mut())
    }
}

#[derive(Debug, Default)]
pub struct ObjRegistry {
    pub model_sets: Vec<ModelSet>,
    pub images: HashMap<String, image::RgbaImage>,
}

impl ObjRegistry {
    pub fn load(
        &mut self,
        path: impl AsRef<Path>,
        load_opts: &tobj::LoadOptions,
    ) -> Result<(), LoadError> {
        let path = path.as_ref();
        tracing::info!("Loading {path:?}");
        tracing::info!("Importing obj...");
        let (models, materials) = match tobj::load_obj(path, load_opts) {
            Err(e) | Ok((_, Err(e))) => return Err(e.into()),
            Ok((models, Ok(materials))) => (models, materials),
        };
        let mut mset = ModelSet {
            models,
            ..Default::default()
        };
        tracing::info!("Registering materials...");
        for material in materials {
            if material.diffuse_texture.is_empty() {
                mset.materials.push(ObjMaterial::Color(
                    material.diffuse[0],
                    material.diffuse[1],
                    material.diffuse[2],
                    material.dissolve,
                ));
            } else {
                let image_path = path.parent().unwrap().join(&material.diffuse_texture);
                let key = image_path.to_str().unwrap().to_owned();
                if !self.images.contains_key(&key) {
                    tracing::info!(
                        "\tLoading diffuse texture for {} from: {:?}",
                        material.name,
                        image_path
                    );
                    self.images.insert(
                        key.clone(),
                        image::open(&image_path)
                            .map_err(|e| LoadError::Image(material.diffuse_texture, image_path, e))?
                            .into_rgba8(),
                    );
                }
                mset.materials.push(ObjMaterial::ImageKey(key));
            }
        }
        Ok(())
    }
}
