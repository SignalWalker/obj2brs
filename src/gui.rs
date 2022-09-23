use std::path::{Path, PathBuf};

use eframe::egui::{CentralPanel, TextEdit};
use eframe::{egui, App};
use egui::special_emojis::GITHUB;
use egui::{
    Button, Color32, Context, Grid, Hyperlink, Label, RichText, Separator, TopBottomPanel, Ui,
};
use rfd::FileDialog;
use uuid::Uuid;

use crate::{BrickType, ConversionOptions, Material};

const BUTTON_COLOR: Color32 = Color32::from_rgb(15, 98, 254);
const ERROR_COLOR: Color32 = Color32::from_rgb(255, 168, 168);
const FOLDER_COLOR: Color32 = Color32::from_rgb(255, 206, 70);
const WINDOW_WIDTH: f32 = 600.;
const WINDOW_HEIGHT: f32 = 480.;

pub fn add_grid(ui: &mut Ui, mut contents: impl FnMut(&mut Ui)) {
    Grid::new("")
        .num_columns(2)
        .spacing([40.0, 4.0])
        .striped(true)
        .show(ui, |ui| contents(ui));
}

pub fn add_horizontal_line(ui: &mut Ui) {
    ui.add(Separator::default().spacing(20.));
}

pub fn info_text(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("❓ You can find your own Brickadia ID by visiting");
        ui.add(Hyperlink::from_label_and_url(
            "brickadia.com/account",
            "https://brickadia.com/account",
        ));
        ui.label("and clicking View Profile");
    });
    ui.label("Your ID will be shown in the URL");
}

pub fn button(ui: &mut Ui, text: &str, enabled: bool) -> bool {
    let text = RichText::new(text).color(Color32::WHITE);
    let b = Button::new(text).fill(BUTTON_COLOR);
    ui.add_enabled(enabled, b)
        .on_hover_text("WARNING! WILL OVERWRITE ANY EXISTING BRS")
        .clicked()
}

pub fn file_button(ui: &mut Ui) -> bool {
    ui.button(RichText::new("🗁").color(FOLDER_COLOR)).clicked()
}

pub fn bool_color(b: bool) -> Color32 {
    if b {
        Color32::WHITE
    } else {
        ERROR_COLOR
    }
}

pub fn footer(ctx: &Context) {
    TopBottomPanel::bottom("footer").show(ctx, |ui: &mut Ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(10.);
            ui.add(Label::new(RichText::new("obj2brs").monospace()));
            ui.label("by Smallguy/Kmschr and Suficio");
            let text = format!("{} {}", GITHUB, "GitHub");
            ui.add(Hyperlink::from_label_and_url(
                text,
                "https://github.com/kmschr/obj2brs",
            ));
            ui.add_space(10.);
        });
    });
}

pub struct Gui {
    input_file_path: String,
    output_directory: String,
    save_owner_id: String,
    conv_opts: ConversionOptions,
}

impl App for Gui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let input_file_valid = Path::new(&self.input_file_path).exists();
        let output_dir_valid = Path::new(&self.output_directory).is_dir();
        let uuid_valid = Uuid::parse_str(&self.save_owner_id).is_ok();
        let can_convert = input_file_valid && output_dir_valid && uuid_valid;

        CentralPanel::default().show(ctx, |ui: &mut Ui| {
            add_grid(ui, |ui| self.paths(ui, input_file_valid, output_dir_valid));
            add_horizontal_line(ui);
            add_grid(ui, |ui| self.options(ui, uuid_valid));
            info_text(ui);

            ui.add_space(10.);
            ui.vertical_centered(|ui| {
                if button(ui, "Voxelize", can_convert) {
                    self.do_conversion()
                }
            });

            footer(ctx);
        });
    }
}

impl Gui {
    pub fn new(
        input_file_path: String,
        output_directory: String,
        conv_opts: ConversionOptions,
    ) -> Self {
        Self {
            input_file_path,
            output_directory,
            save_owner_id: conv_opts.save_owner_id.to_string(),
            conv_opts,
        }
    }

    pub fn run(self) {
        eframe::run_native(
            "obj2brs",
            eframe::NativeOptions {
                initial_window_size: Some([WINDOW_WIDTH, WINDOW_HEIGHT].into()),
                resizable: false,
                icon_data: Some(eframe::IconData {
                    rgba: crate::icon::ICON.to_vec(),
                    width: 32,
                    height: 32,
                }),
                ..Default::default()
            },
            Box::new(|_creation_context| Box::new(self)),
        )
    }

    fn paths(&mut self, ui: &mut Ui, input_file_valid: bool, output_dir_valid: bool) {
        let file_color = bool_color(input_file_valid);

        ui.label("OBJ File").on_hover_text("Model to convert");
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.input_file_path)
                    .desired_width(400.0)
                    .text_color(file_color),
            );
            if file_button(ui) {
                if let Some(path) = FileDialog::new().add_filter("OBJ", &["obj"]).pick_file() {
                    self.input_file_path = path.to_string_lossy().into_owned();
                    self.conv_opts.save_name = match path.file_stem() {
                        Some(s) => s.to_string_lossy().into_owned(),
                        None => self.save_name.clone(),
                    };
                }
            }
        });
        ui.end_row();

        let dir_color = gui::bool_color(output_dir_valid);

        ui.label("Output Directory")
            .on_hover_text("Where generated save will be written to");
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.output_directory)
                    .desired_width(400.0)
                    .text_color(dir_color),
            );
            if gui::file_button(ui) {
                let mut dialog = FileDialog::new();
                if output_dir_valid {
                    dialog = dialog.set_directory(Path::new(self.output_directory.as_str()));
                }

                if let Some(path) = dialog.pick_folder() {
                    self.output_directory = path.to_string_lossy().into_owned();
                }
            }
        });
        ui.end_row();

        ui.label("Save Name")
            .on_hover_text("Name for the brickadia savefile");
        ui.add(TextEdit::singleline(&mut self.save_name));
        ui.end_row();
    }

    fn options(&mut self, ui: &mut Ui, uuid_valid: bool) {
        ui.label("Lossy Conversion").on_hover_text(
            "Whether or not to merge similar bricks to create a less detailed model",
        );
        ui.add_enabled(
            !self.rampify,
            Checkbox::new(&mut self.simplify, "Simplify (reduces brickcount)"),
        );
        ui.end_row();

        ui.label("Raise Underground")
            .on_hover_text("Prevents bricks under the ground plate in Brickadia");
        ui.add(Checkbox::new(&mut self.raise, ""));
        ui.end_row();

        ui.label("Match to Colorset").on_hover_text(
            "Modify the color of the model to match the default color palette in Brickadia",
        );
        ui.add_enabled(
            !self.rampify,
            Checkbox::new(&mut self.match_brickadia_colorset, "Use Default Palette"),
        );
        ui.end_row();

        ui.label("Rampify").on_hover_text(
            "Creates a Lego-World like rampification of the model, uses default colorset",
        );
        ui.add(Checkbox::new(
            &mut self.rampify,
            "Run the result through Wrapperup's plate-rampifier",
        ));
        ui.end_row();

        ui.label("Scale")
            .on_hover_text("Adjusts the overall size of the generated save");
        ui.add(
            DragValue::new(&mut self.scale)
                .min_decimals(2)
                .prefix("x")
                .speed(0.1),
        );
        ui.end_row();

        ui.label("Bricktype")
            .on_hover_text("Which type of bricks will make up the generated save, use default to get a stud texture");
        ui.add_enabled_ui(!self.rampify, |ui| {
            ComboBox::from_label("")
                .selected_text(format!("{:?}", &mut self.bricktype))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.bricktype, BrickType::Microbricks, "Microbricks");
                    ui.selectable_value(&mut self.bricktype, BrickType::Default, "Default");
                    ui.selectable_value(&mut self.bricktype, BrickType::Tiles, "Tiles");
                });
        });
        ui.end_row();

        ui.label("Material");
        ComboBox::from_label("\n")
            .selected_text(format!("{:?}", &mut self.material))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.material, Material::Plastic, "Plastic");
                ui.selectable_value(&mut self.material, Material::Glass, "Glass");
                ui.selectable_value(&mut self.material, Material::Glow, "Glow");
                ui.selectable_value(&mut self.material, Material::Metallic, "Metallic");
                ui.selectable_value(&mut self.material, Material::Hologram, "Hologram");
                ui.selectable_value(&mut self.material, Material::Ghost, "Ghost");
            });
        ui.end_row();

        ui.label("Material Intensity");
        ui.add(Slider::new(
            &mut self.material_intensity,
            RangeInclusive::new(0, 10),
        ));
        ui.end_row();

        let id_color = bool_color(uuid_valid);

        ui.label("Brick Owner")
            .on_hover_text("Who will have ownership of the generated bricks");
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.save_owner_name).desired_width(100.0));
            ui.add(
                TextEdit::singleline(&mut self.save_owner_id)
                    .desired_width(300.0)
                    .text_color(id_color),
            );
        });
        ui.end_row();
    }

    fn do_conversion(&mut self) {
        println!("{:?}", self);
        let mut octree = match generate_octree(self) {
            Ok(tree) => tree,
            Err(e) => {
                println!("{}", e);
                println!("Check that your .mtl file exists and doesn't contain any spaces in the filename!");
                println!("If your .mtl has spaces, rename the file and edit the .obj file to point to the new .mtl file");
                return;
            }
        };

        write_brs_data(&mut octree, self);
    }
}
