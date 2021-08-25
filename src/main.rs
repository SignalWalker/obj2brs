use brickadia as brs;
use tobj;

use std::fs::File;

use cgmath::Vector4;
use image::RgbaImage;

mod barycentric;
mod color;
mod intersect;
mod octree;
mod simplify;
mod voxelize;

use octree::VoxelTree;
use simplify::*;
use voxelize::voxelize;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "obj2brs",
    about = "Voxelizes OBJ files to create textured voxel models"
)]
struct Opt {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    #[structopt(parse(from_os_str))]
    output: PathBuf,
    #[structopt(long, possible_values = &["lossy", "lossless"], default_value = "lossy")]
    simplify: String,
    #[structopt(short, long, default_value = "1")]
    scale: f32,
    #[structopt(short, long, possible_values = &["micro", "normal"], default_value = "normal")]
    bricktype: String,
    #[structopt(short, long, parse(from_occurrences))]
    matchcolor: u8,
}

fn main() {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let mut octree = generate_octree(&opt);

    match opt.output.extension() {
        Some(extension) => {
            match extension.to_str() {
                Some("brs") => write_brs_data(
                    &mut octree,
                    opt.output,
                    opt.simplify,
                    opt.bricktype,
                    opt.matchcolor > 0,
                ),
                // Implement new file types
                Some(extension) => panic!("Output file type {} is not supported", extension),
                None => panic!("Invalid output file type"),
            }
        }
        None => panic!("Invalid output file type"),
    }
}

fn generate_octree(opt: &Opt) -> VoxelTree<Vector4<u8>> {
    match opt.file.extension() {
        Some(extension) => match extension.to_str() {
            Some("obj") => {}
            _ => panic!("Only input files of type obj are supported"),
        },
        None => panic!("Invalid input file type"),
    };

    let file = match opt.file.canonicalize() {
        Err(e) => panic!(
            "Error encountered when looking for file {:?}: {}",
            opt.file,
            e.to_string()
        ),
        Ok(f) => f,
    };

    println!("Importing model...");
    let (mut models, materials) = match tobj::load_obj(&file, true) {
        Err(e) => panic!("Error encountered when loading obj file: {}", e.to_string()),
        Ok(f) => f,
    };

    println!("Loading materials...");
    let mut material_images = Vec::<RgbaImage>::new();
    for material in materials {
        if material.diffuse_texture == "" {
            println!(
                "\tMaterial {} does not have an associated diffuse texture",
                material.name
            );

            // Create mock texture from diffuse color
            let mut image = RgbaImage::new(1, 1);
            image.put_pixel(
                0,
                0,
                image::Rgba([
                    (material.diffuse[0] * 255.) as u8,
                    (material.diffuse[1] * 255.) as u8,
                    (material.diffuse[2] * 255.) as u8,
                    (material.dissolve * 255.) as u8,
                ]),
            );

            material_images.push(image);
        } else {
            let image_path = opt.file.parent().unwrap().join(&material.diffuse_texture);
            println!(
                "\tLoading diffuse texture for {} from: {:?}",
                material.name, image_path
            );

            let image = match image::open(&image_path) {
                Err(e) => panic!(
                    "Error encountered when loading {} texture file from {:?}: {}",
                    &material.diffuse_texture,
                    &image_path,
                    e.to_string()
                ),
                Ok(f) => f.into_rgba8(),
            };
            material_images.push(image);
        }
    }

    println!("Voxelizing...");
    voxelize(
        &mut models,
        &material_images,
        opt.scale,
        opt.bricktype.clone(),
    )
}

fn write_brs_data(
    octree: &mut VoxelTree<Vector4<u8>>,
    output: PathBuf,
    simplify_algo: String,
    bricktype: String,
    match_to_colorset: bool,
) {
    let owner = brs::save::User {
        name: "obj2brs".to_string(),
        id: "00000000-0000-0000-0000-000000000003".parse().unwrap(),
    };

    let reference_save = brs::read::SaveReader::new(File::open("reference.brs").unwrap()).unwrap().read_all_skip_preview().unwrap();

    let mut write_data = brs::save::SaveData {
        header1: brs::save::Header1 {
            author: owner.clone(),
            host: Some(owner.clone()),
            ..Default::default()
        },
        header2: brs::save::Header2 {
            brick_assets: vec!["PB_DefaultMicroBrick".into(), "PB_DefaultBrick".into()],
            brick_owners: vec![brs::save::BrickOwner::from_user_bricks(owner.clone(), 1)],
            colors: reference_save.header2.colors,
            ..Default::default()
        },
        ..Default::default()
    };

    println!("Simplifying {:?}...", simplify_algo);
    if simplify_algo == "lossless" {
        simplify_lossless(octree, &mut write_data, bricktype, match_to_colorset);
    } else {
        simplify(octree, &mut write_data, bricktype, match_to_colorset);
    }

    // Write file
    println!("Writing file...");
    brs::write::SaveWriter::new(File::create(output).unwrap(), write_data)
        .write()
        .unwrap();
}
