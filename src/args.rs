use clap::{Parser, Subcommand, ValueEnum};

use crate::bert;

#[derive(Subcommand, Default)]
pub enum Action {
    #[default]
    Load,
    Save,
}

#[derive(Default, ValueEnum, Clone, Debug)]
pub enum Model {
    #[default]
    MiniLm,
    GteSmall,
    GteBase,
    E5Large,
}

impl Model {
    pub fn get_path(&self) -> &'static str {
        match self {
            Model::MiniLm => "models/all-MiniLM-L6-v2",
            Model::GteSmall => "models/gte-small",
            Model::GteBase => "models/gte-base",
            Model::E5Large => "models/e5-large",
        }
    }
    pub fn get_config(&self) -> bert::Config {
        match self {
            Model::MiniLm => bert::Config::minilm_l6(),
            Model::GteSmall => bert::Config::gte_small(),
            Model::GteBase => bert::Config::gte_base(),
            Model::E5Large => bert::Config::e5_large(),
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub action: Action,

    #[arg(value_enum, short, long, default_value_t = Model::default())]
    pub model: Model,

    #[arg(short, long)]
    pub query: String,

    #[arg(short, long, default_value_t = 8192)]
    pub batch: usize,

    #[arg(short = 'k', long, default_value_t = 256)]
    pub top_k: usize,
    
    /// (this is kind of model-dependent)
    #[arg(short = 'p', long, default_value_t = 0.35)]
    pub min_p: f32,
}

pub fn get_args() -> Args {
    Args::parse()
}
