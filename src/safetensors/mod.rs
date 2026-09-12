pub mod reader;

pub const ST_MODEL_PATH: &str = "model.safetensors";

pub use reader::SafeTensors;

use crate::{Config, args, tokenizer::VOCAB_PATH};

pub fn get_model_path(model: args::Model) -> (String, String, Config) {
    let pdir = model.get_path();
    let config = model.get_config();
    (
        format!("{}/{ST_MODEL_PATH}", pdir),
        format!("{}/{VOCAB_PATH}", pdir),
        config,
    )
}
