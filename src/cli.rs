use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use lazy_static::lazy_static;
use uuid::Uuid;

use crate::{BrickType, ConversionOptions, LogFormat, Material};

lazy_static! {
    pub static ref BUILD_DIR: PathBuf = {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir()
                .unwrap()
                .join("Brickadia\\Saved\\Builds")
        }
        #[cfg(target_os = "linux")]
        {
            dirs::config_dir()
                .unwrap()
                .join("Epic/Brickadia/Saved/Builds")
        }
    };
}

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(short, long, default_value = "warn,obj2brs=info", env = "RUST_LOG")]
    /// Logging output filters; comma-separated
    pub log_filter: String,
    #[clap(long, value_parser, default_value = "pretty")]
    /// Logging output format
    pub log_format: LogFormat,
    #[clap(long, default_value_t = num_cpus::get())]
    pub threads: usize,
    #[clap(long)]
    /// Enable the GUI, even if an input file is given on the CLI
    pub gui: bool,
    #[clap(long)]
    /// Overwrite existing files
    pub overwrite: bool,
    #[clap(short, long)]
    /// Prevent placing bricks underground
    pub raise: bool,
    #[clap(short, long, default_value_t = 1.00)]
    /// Scale factor from the input model to the output save
    pub scale: f32,
    #[clap(long, value_parser, default_value = "plastic")]
    /// Material type for output bricks
    pub material: Material,
    #[clap(short = 'i', long = "intensity", default_value_t = 5, value_parser = clap::value_parser!(u32).range(0..10))]
    /// Intensity of output brick material
    pub material_intensity: u32,
    #[clap(long, default_value = "obj2brs")]
    pub owner_name: String,
    #[clap(long, default_value = "d66c4ad5-59fc-4a9b-80b8-08dedc25bff9")]
    pub owner_id: Uuid,
    #[clap(short, long, value_parser, default_value = BUILD_DIR.to_str().unwrap())]
    /// Output file or directory. Must be a directory if more than one input file is specified.
    pub output: PathBuf,
    #[clap(subcommand)]
    /// Subcommand
    pub command: Option<Command>,
}

impl Cli {
    pub fn inputs(&self) -> &[PathBuf] {
        match self.command {
            None => &[],
            Some(Command::Convert { ref inputs, .. } | Command::Rampify { ref inputs, .. }) => {
                inputs
            }
        }
    }

    // pub fn lossy(&self) -> bool {
    //     match self.command {
    //         None => false,
    //         Some(Command::Rampify { .. }) => false,
    //         Some(Command::Convert { lossy, .. }) => lossy,
    //     }
    // }

    // pub fn default_colorset(&self) -> bool {
    //     match self.command {
    //         None => false,
    //         Some(Command::Rampify { .. }) => true,
    //         Some(Command::Convert { default_colorset, .. }) => default_colorset,
    //     }
    // }

    // pub fn bricktype(&self) -> BrickType {
    //     match self.command {
    //         None => BrickType::Microbricks,
    //         Some(Command::Rampify { .. }) => BrickType::Default,
    //         Some(Command::Convert { bricktype, .. }) => bricktype,
    //     }
    // }

    // pub fn max_merge(&self) -> u32 {
    //     match self.command {
    //         None => 200,
    //         Some(Command::Rampify { .. }) => 1,
    //         Some(Command::Convert { max_merge, .. }) => max_merge,
    //     }
    // }
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    #[clap()]
    /// Standard conversion
    Convert {
        #[clap(long)]
        /// Perform lossy simplification, which results in a less detailed build
        lossy: bool,
        #[clap(long)]
        /// Match model colors to the default Brickadia colorset
        default_colorset: bool,
        #[clap(short, long, value_parser, default_value = "microbricks")]
        /// Type of bricks to use in the generated save. Use "default" to get a stud texture.
        bricktype: BrickType,
        #[clap(long, default_value_t = 200)]
        /// Maximum merges to perform when simplifying
        max_merge: u32,
        #[clap(value_parser)]
        /// Input files. If empty, launch the GUI.
        inputs: Vec<PathBuf>,
    },
    #[clap()]
    /// Convert & apply Wrapperup's plate-rampifier
    Rampify {
        #[clap(value_parser)]
        /// Input files. If empty, launch the GUI.
        inputs: Vec<PathBuf>,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::Convert {
            lossy: false,
            default_colorset: false,
            bricktype: BrickType::Microbricks,
            max_merge: 200,
            inputs: vec![],
        }
    }
}

impl Command {
    pub fn as_conversion_options(&self) -> ConversionOptions {
        match self {
            Self::Convert {
                lossy,
                default_colorset,
                bricktype,
                max_merge,
                ..
            } => ConversionOptions::Simplify {
                lossless: !lossy,
                match_default_colorset: *default_colorset,
                bricktype: *bricktype,
                max_merge: *max_merge,
            },
            Self::Rampify { .. } => ConversionOptions::Rampify {},
        }
    }
}
