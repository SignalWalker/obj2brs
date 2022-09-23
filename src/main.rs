mod barycentric;
mod cli;
mod color;
// mod gui;
mod icon;
mod intersect;
mod octree;
mod palette;
mod rampify;
mod simplify;
mod voxelize;

use brickadia as brs;
use brs::save::Preview;
use cgmath::Vector4;
use clap::{CommandFactory, Parser};
use eframe::{egui, egui::*, run_native, App, NativeOptions};
use rayon::prelude::{IntoParallelRefIterator, IntoParallelIterator, ParallelIterator};
// use gui::bool_color;
use rfd::FileDialog;
use simplify::*;
use std::{
    fs::File,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use voxelize::voxelize;

const OBJ_ICON: &[u8; 10987] = include_bytes!("../res/obj_icon.png");

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, clap::ValueEnum)]
pub enum BrickType {
    Microbricks,
    Default,
    Tiles,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, clap::ValueEnum)]
pub enum Material {
    Plastic,
    Glass,
    Glow,
    Metallic,
    Hologram,
    Ghost,
}

#[derive(thiserror::Error, Debug)]
pub enum ConversionError {
    #[error(transparent)]
    LoadObj(#[from] tobj::LoadError),
    #[error("Failed to load {0} texture file from {1:?}: {2}")]
    LoadImg(String, PathBuf, image::ImageError),
}

fn load_model(path: &Path) -> Result<(Vec<tobj::Model>, Vec<image::RgbaImage>), ConversionError> {
    tracing::info!("Loading {path:?}");
    tracing::info!("Importing model...");
    let (models, materials) = match tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            ..Default::default()
        },
    ) {
        Err(e) | Ok((_, Err(e))) => return Err(e.into()),
        Ok((models, Ok(materials))) => (models, materials),
    };
    tracing::info!("Loading materials...");
    let mut material_images = Vec::<image::RgbaImage>::new();
    for material in materials {
        if material.diffuse_texture.is_empty() {
            tracing::info!(
                "\tMaterial {} does not have an associated diffuse texture",
                material.name
            );

            // Create mock texture from diffuse color
            let mut image = image::RgbaImage::new(1, 1);

            image.put_pixel(
                0,
                0,
                image::Rgba([
                    color::ftoi(material.diffuse[0]),
                    color::ftoi(material.diffuse[1]),
                    color::ftoi(material.diffuse[2]),
                    color::ftoi(material.dissolve),
                ]),
            );

            material_images.push(image);
        } else {
            let image_path = path.parent().unwrap().join(&material.diffuse_texture);
            tracing::info!(
                "\tLoading diffuse texture for {} from: {:?}",
                material.name,
                image_path
            );

            let image = image::open(&image_path).map_err(|e| ConversionError::LoadImg(material.diffuse_texture, image_path, e))?.into_rgba8();
            material_images.push(image);
        }
    }
    Ok((models, material_images))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ConversionOptions {
    Simplify {
        lossless: bool,
        match_default_colorset: bool,
        bricktype: BrickType,
        max_merge: u32,
    },
    Rampify {},
}

fn start_brs_data(owner: &brs::save::User, material: Material) -> brs::save::SaveData {
    brs::save::SaveData {
        header1: brs::save::Header1 {
            author: owner.clone(),
            host: Some(owner.clone()),
            ..Default::default()
        },
        header2: brs::save::Header2 {
            brick_assets: vec![
                "PB_DefaultMicroBrick".into(),
                "PB_DefaultBrick".into(),
                "PB_DefaultRamp".into(),
                "PB_DefaultWedge".into(),
                "PB_DefaultTile".into(),
            ],
            materials: match material {
                Material::Plastic => vec!["BMC_Plastic".into()],
                Material::Glass => vec!["BMC_Glass".into()],
                Material::Glow => vec!["BMC_Glow".into()],
                Material::Metallic => vec!["BMC_Metallic".into()],
                Material::Hologram => vec!["BMC_Hologram".into()],
                Material::Ghost => vec!["BMC_Ghost".into()],
            },
            brick_owners: vec![brs::save::BrickOwner::from_user_bricks(owner.clone(), 1)],
            colors: palette::DEFAULT_PALETTE.to_vec(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn write_octree(
    octree: &mut octree::VoxelTree<Vector4<u8>>,
    write_data: &mut brs::save::SaveData,
    material_intensity: u32,
    options: ConversionOptions,
) {
    if let ConversionOptions::Simplify {
        bricktype: BrickType::Tiles,
        ..
    } = options
    {
        write_data.header2.brick_assets[1] = "PB_DefaultTile".into();
    }

    tracing::info!("Simplifying...");
    match options {
        ConversionOptions::Simplify {
            lossless: true,
            max_merge,
            match_default_colorset,
            bricktype,
        } => simplify_lossless(
            octree,
            write_data,
            match_default_colorset,
            bricktype,
            material_intensity,
            max_merge as isize,
        ),
        ConversionOptions::Simplify {
            lossless: false,
            max_merge,
            match_default_colorset,
            bricktype,
        } => simplify_lossy(
            octree,
            write_data,
            match_default_colorset,
            bricktype,
            material_intensity,
            max_merge as isize,
        ),
        ConversionOptions::Rampify { .. } => {
            simplify_lossless(
                octree,
                write_data,
                true,
                BrickType::Default,
                material_intensity,
                1,
            );
            rampify::rampify(write_data);
        }
    }
}

fn raise_brs(data: &mut brs::save::SaveData) {
    tracing::info!("Raising...");
    let mut min_z = 0;
    for brick in &data.bricks {
        let height = match brick.size {
            brs::save::Size::Procedural(_x, _y, z) => z,
            _ => 0,
        };
        let z = brick.position.2 - height as i32;
        if z < min_z {
            min_z = z;
        }
    }

    for brick in &mut data.bricks {
        brick.position.2 -= min_z;
    }
}

fn write_brs(data: brs::save::SaveData, path: &Path) {
    // Write file
    tracing::info!("Writing {} bricks to {path:?}...", data.bricks.len());
    tracing::info!("Save Written!");
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, clap::ValueEnum)]
pub enum LogFormat {
    Compact,
    Full,
    Pretty,
    Json,
}

fn init_tracing(format: LogFormat, filter: &str) {
    let tsub = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_timer(tracing_subscriber::fmt::time::OffsetTime::new(
            time::UtcOffset::current_local_offset().expect("couldn't get local time offset"),
            time::macros::format_description!("[hour]:[minute]:[second]"),
        ))
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_env_filter(filter);

    match format {
        LogFormat::Compact => tsub.compact().init(),
        LogFormat::Full => tsub.init(),
        LogFormat::Pretty => tsub.pretty().init(),
        LogFormat::Json => tsub.json().init(),
    }
}

lazy_static::lazy_static! {
    pub static ref PREVIEW_BYTES: Vec<u8> = {
        let img = image::load_from_memory_with_format(OBJ_ICON, image::ImageFormat::Png).unwrap();

        let mut bytes = Vec::new();
        img
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageOutputFormat::Png,
            )
            .unwrap();
        bytes
    };
}

fn voxelize_obj(
    path: &Path,
    scale: f32,
    bricktype: BrickType,
) -> Result<octree::VoxelTree<Vector4<u8>>, ConversionError> {
    tracing::info!("Voxelizing {path:?}");
    let (mut models, material_images) = load_model(path)?; //.expect(&format!("Failed to load input model: {input:?}"));
    Ok(voxelize(&mut models, &material_images, scale, bricktype))
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("Output file already exists & overwriting is disabled")]
    Collision,
    #[error(transparent)]
    Conversion(#[from] ConversionError),
}

fn write_objs_to_brs(
    owner: &brs::save::User,
    material: Material,
    material_intensity: u32,
    raise: bool,
    scale: f32,
    conv_opts: ConversionOptions,
    preview: Preview,
    overwrite: bool,
    inputs: &[impl AsRef<Path>],
    output: impl AsRef<Path>,
) -> Result<(), WriteError> {
    let output = output.as_ref();
    match (output.exists(), overwrite) {
        (false, _) => {}
        (true, false) => return Err(WriteError::Collision),
        (true, true) => tracing::warn!("Overwriting {output:?}"),
    }
    let mut data = start_brs_data(&owner, material);
    for input in inputs.iter().map(|p| p.as_ref()) {
        tracing::info!("Adding {input:?} to brs data");
        let mut octree = voxelize_obj(
            input,
            scale,
            match conv_opts {
                ConversionOptions::Rampify {} => BrickType::Default,
                ConversionOptions::Simplify { bricktype, .. } => bricktype,
            },
        )?;
        write_octree(&mut octree, &mut data, material_intensity, conv_opts);
    }
    if raise {
        raise_brs(&mut data);
    }
    data.preview = preview;
    brs::write::SaveWriter::new(File::create(output).unwrap(), data)
        .write()
        .unwrap();
    Ok(())
}

fn main() {
    let mut args = cli::Cli::parse();
    init_tracing(args.log_format, &args.log_filter);
    rayon::ThreadPoolBuilder::new().num_threads(args.threads).build_global().unwrap();

    if args.gui || args.inputs().is_empty() {
        // gui::Gui::new(
        //     args.inputs()
        //         .get(0)
        //         .map_or("test.obj", |i| i.to_str().unwrap())
        //         .to_owned(),
        //     match args.output.is_dir() {
        //         true => args.output.to_str().unwrap().to_owned(),
        //         false => todo!(),
        //     },
        //     todo!(),
        // )
        // .run();
    } else {
        let brs_owner = brs::save::User {
            name: args.owner_name.clone(),
            id: args.owner_id,
        };
        let conv_opts = args.command.as_ref().unwrap().as_conversion_options();
        if args.output.is_dir() {
            // write all converted inputs to separate files in args.output
            args.inputs().into_par_iter().for_each(|input| {
                let file_name = input.file_stem().unwrap().to_str().unwrap().to_owned();
                let file_path = args.output.join(format!("{}.brs", file_name));
                tracing::info!("Generating {file_name:?}.brs...");
                match write_objs_to_brs(
                    &brs_owner,
                    args.material,
                    args.material_intensity,
                    args.raise,
                    args.scale,
                    conv_opts,
                    Preview::PNG(PREVIEW_BYTES.clone()),
                    args.overwrite,
                    &[input],
                    &file_path,
                ) {
                    Ok(_) => {}
                    Err(WriteError::Collision) => tracing::error!("{file_path:?} exists; skipping..."),
                    Err(e) => tracing::error!("{e:?}")
                };
            });
        } else {
            // write all converted inputs to a single file (args.output)
            tracing::info!("Generating {0:?}...", args.output);
            write_objs_to_brs(
                &brs_owner,
                args.material,
                args.material_intensity,
                args.raise,
                args.scale,
                conv_opts,
                Preview::PNG(PREVIEW_BYTES.clone()),
                args.overwrite,
                args.inputs(),
                &args.output,
            )
            .unwrap();
        }
    }
}
