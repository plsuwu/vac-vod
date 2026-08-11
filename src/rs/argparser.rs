use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::OnceLock;

use clap::{Parser, ValueEnum};
use dirs::document_dir;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum ActionArg {
    #[value(alias("init"))]
    Manifest,
    Fetch,
    Ingest,
    All,
}

static CLI_ARGS: LazyLock<OnceLock<Arc<CLIArgs>>> = LazyLock::new(OnceLock::new);

pub fn get_args() -> Arc<CLIArgs> {
    let args = CLI_ARGS.get_or_init(|| Arc::new(CLIArgs::parse()));
    Arc::clone(args)
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CLIArgs {
    /// List of YouTube Channel IDs to fetch captions for.
    pub channel_ids: Vec<String>,

    /// Job to run for this Channel
    #[arg(value_enum, short, long, default_value_t = ActionArg::All)]
    pub action: ActionArg,

    /// Base directory for outputs.
    #[arg(short, long, default_value_t = default_outdir())]
    pub output_dir: String,

    /// Number of fetch jobs to run at once.
    #[arg(short, long, default_value_t = 10)]
    pub concurrency: usize,
}

fn default_outdir() -> String {
    document_dir()
        .map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or(String::from("."))
}
