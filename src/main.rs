mod barycentric;
mod color;
mod intersect;
mod octree;
mod palette;
mod simplify;
mod voxelize;

use brickadia as brs;
use cgmath::Vector4;
use eframe::{run_native, NativeOptions, epi::App};
use egui::{color::*, *};
use simplify::*;
use std::fs::File;
use std::{env, path::{Path, PathBuf}};
use structopt::StructOpt;
use voxelize::voxelize;

const WINDOW_WIDTH: f32 = 600.0;
const WINDOW_HEIGHT: f32 = 600.0;

const ERROR_COLOR: Color32 = Color32::from_rgb(255, 168, 168);
const FOLDER_COLOR: Color32 = Color32::from_rgb(255, 206, 70);

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

#[derive(Debug, PartialEq)]
enum BrickType {
    Microbricks,
    Default
}

struct Obj2Brs {
    file: String,
    output: String,
    simplify: bool,
    scale: f32,
    bricktype: BrickType,
    raise: bool,
}

impl Default for Obj2Brs {
    fn default() -> Self {
        Self {
            file: "test.obj".into(),
            output: "builds".into(),
            simplify: true,
            scale: 1.0,
            bricktype: BrickType::Microbricks,
            raise: true,
        }
    }
}

impl App for Obj2Brs {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut eframe::epi::Frame<'_>) {
        CentralPanel::default().show(ctx, |ui: &mut Ui| {
            Grid::new("paths grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.paths(ui);
                });

            ui.add(Separator::default().spacing(20.));

            Grid::new("options grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.options(ui);
                });

            ui.add(Separator::default().spacing(20.));
            
            ui.vertical_centered(|ui| {
                if ui.button("Voxelize").clicked() {
                    if Path::new(&self.file).exists() && Path::new(&self.output).exists() {
                        let filename = Path::new(&self.file).file_stem().unwrap();
    
                        let file = PathBuf::from(self.file.clone());
                        let output = PathBuf::from(self.output.clone() + "/" + filename.to_str().unwrap() + ".brs");
                        let simplify = if self.simplify {
                            "lossy".into()
                        } else {
                            "lossless".into()
                        };
                        let bricktype = match &self.bricktype {
                            BrickType::Microbricks => "micro".into(),
                            BrickType::Default => "normal".into(),
                        };
    
                        let opt = Opt {
                            file,
                            output,
                            simplify,
                            scale: self.scale,
                            bricktype,
                            matchcolor: 0,
                        };
                        run(opt);
                    }
                }
            });
            
            self.footer(ctx);
        });
    }

    fn name(&self) -> &str {
        "obj2brs"
    }
}

impl Obj2Brs {
    fn paths(&mut self, ui: &mut Ui) {
        let Self {
            file,
            output,
            ..
        } = self;

        let file_color = if Path::new(&file).exists() {
            Color32::WHITE
        } else {
            ERROR_COLOR
        };

        ui.label("OBJ File").on_hover_text("Model to convert");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(file).desired_width(400.0).text_color(file_color));
            if ui.button(RichText::new("🗁").color(FOLDER_COLOR)).clicked() {
                match nfd::open_file_dialog(Some("obj"), None).unwrap() {
                    nfd::Response::Okay(file_path) => {
                        file.clear();
                        file.push_str(file_path.as_str());
                    },
                    _ => ()
                }
            }
        });
        ui.end_row();

        let dir_color = if Path::new(&output).exists() {
            Color32::WHITE
        } else {
            ERROR_COLOR
        };

        ui.add(Label::new("Output Directory"));
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(output).desired_width(400.0).text_color(dir_color));
            if ui.button(egui::RichText::new("🗁").color(FOLDER_COLOR)).clicked() {
                let default_dir = if Path::new(&output).exists() {
                    Some(output.as_str())
                } else {
                    None
                };

                match nfd::open_pick_folder(default_dir).unwrap() {
                    nfd::Response::Okay(file_path) => {
                        output.clear();
                        output.push_str(file_path.as_str());
                    },
                    _ => ()
                }
            }
        });
        ui.end_row();
    }

    fn options(&mut self, ui: &mut Ui) {
        let Self {
            simplify,
            scale,
            bricktype,
            raise,
            ..
        } = self;

        ui.label("Lossy Conversion");
        ui.add(Checkbox::new(simplify, "Simplify (greatly reduces brickcount)"));
        ui.end_row();

        ui.label("Scale");
        ui.add(DragValue::new(scale).min_decimals(2).prefix("x").speed(0.1));
        ui.end_row();

        ui.label("Bricktype");
        ComboBox::from_label("")
            .selected_text(format!("{:?}", bricktype))
            .show_ui(ui, |ui| {
                ui.selectable_value(bricktype, BrickType::Microbricks, "Microbricks");
                ui.selectable_value(bricktype, BrickType::Default, "Default");
            });
        ui.end_row();

        ui.label("Raise Underground");
        ui.add(Checkbox::new(raise, ""));
        ui.end_row();
    }

    fn footer(&mut self, ctx: &CtxRef) {
        TopBottomPanel::bottom("footer").show(ctx, |ui: &mut Ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.);
                ui.add(Label::new(RichText::new("obj2brs").monospace()));
                ui.add(Label::new("by Smallguy/Kmschr and French Fries/CheezBarger"));
                ui.add(Hyperlink::from_label_and_url(format!("{} {}", special_emojis::GITHUB, "GitHub"), "https://github.com/kmschr/obj2brs"));
                ui.add_space(10.);
            });
        });
    }
}

fn main() {
    let build_dir = match env::consts::OS {
        "windows" => dirs::data_local_dir().unwrap().to_str().unwrap().to_string() + "\\Brickadia\\Saved\\Builds",
        "linux" => dirs::config_dir().unwrap().to_str().unwrap().to_string() + "/Epic/Brickadia/Saved/Builds",
        _ => String::new(),
    };

    let app = Obj2Brs {
        output: build_dir,
        ..Default::default()
    };
    let win_option = NativeOptions {
        initial_window_size: Some([WINDOW_WIDTH, WINDOW_HEIGHT].into()),
        resizable: false,
        ..Default::default()
    };
    run_native(Box::new(app), win_option);
}

fn run(opt: Opt) {
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

fn generate_octree(opt: &Opt) -> octree::VoxelTree<Vector4<u8>> {
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
    let mut material_images = Vec::<image::RgbaImage>::new();
    for material in materials {
        if material.diffuse_texture == "" {
            println!(
                "\tMaterial {} does not have an associated diffuse texture",
                material.name
            );

            // Create mock texture from diffuse color
            let mut image = image::RgbaImage::new(1, 1);
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
    octree: &mut octree::VoxelTree<Vector4<u8>>,
    output: PathBuf,
    simplify_algo: String,
    bricktype: String,
    match_to_colorset: bool,
) {
    let owner = brs::save::User {
        name: "obj2brs".to_string(),
        id: "00000000-0000-0000-0000-000000000420".parse().unwrap(),
    };

    let mut write_data = brs::save::SaveData {
        header1: brs::save::Header1 {
            author: owner.clone(),
            host: Some(owner.clone()),
            ..Default::default()
        },
        header2: brs::save::Header2 {
            brick_assets: vec!["PB_DefaultMicroBrick".into(), "PB_DefaultBrick".into()],
            brick_owners: vec![brs::save::BrickOwner::from_user_bricks(owner.clone(), 1)],
            colors: palette::default_palette(),
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
