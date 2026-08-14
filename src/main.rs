use crate::rs::prelude::*;

mod rs;

pub type Result<T> = core::result::Result<T, &'static str>;

#[tokio::main]
async fn main() {
    init_stdout_logger();

    // let channel_ids: [String; 2] = [
    //     "UCaZkRdEEpePJ4EEZznuqh8g".to_string(),
    //     "UCBustguC_fsnqDQZxOBgGEg".to_string(),
    // ];

    let args = get_args();
    let channel_ids = &args.channel_ids;
    let base_outdir = &args.output_dir;
    let query = args.search.clone().unwrap();
    let k = args.k;

    _ = match get_args().action {
        ActionArg::Manifest => run_manifest_action(channel_ids, base_outdir).await,
        ActionArg::Fetch => run_fetch_action(channel_ids, base_outdir).await,
        ActionArg::Ingest => todo!(),
        ActionArg::All => todo!(),
        ActionArg::Eval => run_eval_action(&query, k),
    };
}

async fn run_fetch_action(channel_ids: &[String], base_outdir: &str) -> Result<()> {
    let mut failures = Vec::new();
    for channel_id in channel_ids {
        let failed = fetch_uploads(channel_id, base_outdir, 15).await;
        failures.extend(failed);
    }

    tracing::info!(
        total_items = channel_ids.len(),
        failed_count = failures.len(),
        failures = ?failures,
        "complete"
    );
    Ok(())
}

fn run_eval_action(query: &str, k: usize) -> Result<()> {
    tracing::info!(query);
    crate::rs::eval::search(query, k);

    Ok(())
}

async fn run_manifest_action(_channel_ids: &[String], _base_outdir: &str) -> Result<()> {
    unimplemented!("Use the JS implementation for now (`bun run fetch`)");
}

fn init_stdout_logger() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .pretty()
        .init();
}
