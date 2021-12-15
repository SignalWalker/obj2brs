mod barycentric;
mod color;
mod icon;
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
use uuid::Uuid;
use std::ffi::OsString;
use std::fs::File;
use std::{env, path::Path};
use voxelize::voxelize;

const WINDOW_WIDTH: f32 = 600.0;
const WINDOW_HEIGHT: f32 = 420.0;

const ERROR_COLOR: Color32 = Color32::from_rgb(255, 168, 168);
const FOLDER_COLOR: Color32 = Color32::from_rgb(255, 206, 70);
const BLUE: Color32 = Color32::from_rgb(15, 98, 254);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BrickType {
    Microbricks,
    Default
}

#[derive(Debug)]
struct Obj2Brs {
    input_file_path: String,
    output_directory: String,
    save_name: String,
    simplify: bool,
    scale: f32,
    bricktype: BrickType,
    matchcolor: bool,
    raise: bool,
    owner_name: String,
    owner_id: String,
}

impl Default for Obj2Brs {
    fn default() -> Self {
        Self {
            input_file_path: "test.obj".into(),
            output_directory: "builds".into(),
            save_name: "test".into(),
            simplify: true,
            scale: 1.0,
            bricktype: BrickType::Microbricks,
            matchcolor: false,
            raise: true,
            owner_name: "obj2brs".into(),
            owner_id: "d66c4ad5-59fc-4a9b-80b8-08dedc25bff9".into(),
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

            ui.horizontal(|ui| {
                ui.label("❓ You can find your own Brickadia ID by visiting");
                ui.add(Hyperlink::from_label_and_url("brickadia.com/account", "https://brickadia.com/account"));
                ui.label("and clicking View Profile");
            });

            ui.label("Your ID will be shown in the URL");
            
            ui.add_space(20.);
            ui.vertical_centered(|ui| {
                if ui.add(Button::new(RichText::new("Voxelize").color(Color32::WHITE)).fill(BLUE)).clicked() {
                    if Path::new(&self.input_file_path).exists() && Path::new(&self.output_directory).exists() {
                        self.run()
                    }
                }
                ui.label("WARNING! WILL OVERWRITE ANY EXISTING BRS IN OUTPUT DIRECTORY")
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
            input_file_path: file,
            output_directory: output,
            save_name,
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
                        let old = save_name.clone();
                        save_name.clear();
                        save_name.push_str(Path::new(&file_path.as_str()).file_stem().unwrap_or(&OsString::from(old)).to_str().unwrap());
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

        ui.label("Output Directory").on_hover_text("Where generated save will be written to");
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

        ui.label("Save Name").on_hover_text("Name for the brickadia savefile");
        ui.add(TextEdit::singleline(save_name));
        ui.end_row();
    }

    fn options(&mut self, ui: &mut Ui) {
        let Self {
            simplify,
            scale,
            bricktype,
            raise,
            matchcolor,
            owner_name: name,
            owner_id: uuid,
            ..
        } = self;

        ui.label("Lossy Conversion").on_hover_text("Whether or not to merge similar bricks to create a less detailed model");
        ui.add(Checkbox::new(simplify, "Simplify (greatly reduces brickcount)"));
        ui.end_row();

        ui.label("Raise Underground").on_hover_text("Prevents parts of the model from loading under the ground plate in Brickadia");
        ui.add(Checkbox::new(raise, ""));
        ui.end_row();

        ui.label("Match to Colorset").on_hover_text("Modify the color of the model to match the default color palette in Brickadia");
        ui.add(Checkbox::new(matchcolor, "Use Default Palette"));
        ui.end_row();

        ui.label("Scale").on_hover_text("Adjusts the overall size of the generated save, not the size of the individual bricks");
        ui.add(DragValue::new(scale).min_decimals(2).prefix("x").speed(0.1));
        ui.end_row();

        ui.label("Bricktype").on_hover_text("Which type of bricks will make up the generated save, use default to get a stud texture");
        ComboBox::from_label("")
            .selected_text(format!("{:?}", bricktype))
            .show_ui(ui, |ui| {
                ui.selectable_value(bricktype, BrickType::Microbricks, "Microbricks");
                ui.selectable_value(bricktype, BrickType::Default, "Default");
            });
        ui.end_row();

        let id_color = match Uuid::parse_str(uuid) {
            Ok(_id) => Color32::WHITE,
            Err(_e) => ERROR_COLOR,
        };

        ui.label("Brick Owner").on_hover_text("Who will have ownership of the generated bricks");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(name).desired_width(100.0));
            ui.add(TextEdit::singleline(uuid).desired_width(300.0).text_color(id_color));
        });
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

    fn run(&self) {
        println!("{:?}", self);
        let mut octree = generate_octree(self);

        write_brs_data(
            &mut octree,
            &self,
        );
    }
}

fn generate_octree(opt: &Obj2Brs) -> octree::VoxelTree<Vector4<u8>> {
    let file = match Path::new(&opt.input_file_path).canonicalize() {
        Err(e) => panic!(
            "Error encountered when looking for file {:?}: {}",
            opt.input_file_path,
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
            let image_path = Path::new(&opt.input_file_path).parent().unwrap().join(&material.diffuse_texture);
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
        opt.bricktype,
    )
}

fn write_brs_data(
    octree: &mut octree::VoxelTree<Vector4<u8>>,
    opts: &Obj2Brs,
) {
    let owner = brs::save::User {
        name: opts.owner_name.clone(),
        id: opts.owner_id.parse().unwrap(),
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
            colors: palette::DEFAULT_PALETTE.to_vec(),
            ..Default::default()
        },
        ..Default::default()
    };

    if opts.simplify {
        simplify_lossy(octree, &mut write_data, opts.bricktype, opts.matchcolor);
    } else {
        simplify_lossless(octree, &mut write_data, opts.bricktype, opts.matchcolor);
    }

    if opts.raise {
        let mut min_z = 0;
        for brick in &write_data.bricks {
            let height = match brick.size {
                brs::save::Size::Procedural(_x, _y, z) => z,
                _ => 0
            };
            let z = brick.position.2 - height as i32;
            if z < min_z {
                min_z = z;
            }
        }

        for brick in &mut write_data.bricks {
            brick.position.2 -= min_z;
        }
    }

    // Write file
    println!("Writing file...");
    
    let output_file_path = opts.output_directory.clone() + "/" + &opts.save_name + ".brs";
    brs::write::SaveWriter::new(File::create(output_file_path).unwrap(), write_data)
        .write()
        .unwrap();

    println!("Save Written!");
}

fn main() {
    let build_dir = match env::consts::OS {
        "windows" => dirs::data_local_dir().unwrap().to_str().unwrap().to_string() + "\\Brickadia\\Saved\\Builds",
        "linux" => dirs::config_dir().unwrap().to_str().unwrap().to_string() + "/Epic/Brickadia/Saved/Builds",
        _ => String::new(),
    };

    let app = Obj2Brs {
        output_directory: build_dir,
        ..Default::default()
    };
    let win_option = NativeOptions {
        initial_window_size: Some([WINDOW_WIDTH, WINDOW_HEIGHT].into()),
        resizable: false,
        icon_data: Some(eframe::epi::IconData {
            rgba: icon::ICON.to_vec(),
            width: 32,
            height: 32,
        }),
        ..Default::default()
    };
    run_native(Box::new(app), win_option);
}
