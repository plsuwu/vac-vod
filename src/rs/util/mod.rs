use serde::Deserialize;

use crate::rs::util::env::{Error as EnvError, from_env};

pub mod env;
pub mod paths;
pub mod tracing;

#[derive(Debug)]
pub enum Var {
    YoutubeApiKey,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Env {
    pub youtube_api_key: String,
}

impl Env {
    pub fn get(var: Var) -> Result<String, EnvError> {
        let vars = from_env::<Env>()?;
        Ok(match var {
            Var::YoutubeApiKey => vars.youtube_api_key.clone(),
        })
    }
}

#[macro_export]
macro_rules! var {
    ($var:expr) => {
        $crate::rs::util::Env::get($var)
    };
}
