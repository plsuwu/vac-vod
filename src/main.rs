mod args;
mod bert;
mod ingest;
mod misc;
mod safetensors;
mod tensor;
mod tokenizer;

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};

use tokenizer::wordpiece::WordPiece;
use tracing::{Level, instrument};

use crate::args::get_args;
use crate::bert::Config;
use crate::bert::batched::embed_all_gpu;
use crate::bert::gpu::GpuBert;
use crate::ingest::Chunk;
use crate::misc::init_log;
use crate::safetensors::{ST_MODEL_PATH, SafeTensors, get_model_path};
use crate::tensor::dot;

const EMBEDDINGS_OUTPATH: &str = ".embeddings";
const DEFAULT_DEDUP_GAP_SECS: f64 = 60.0;

pub struct Index {
    pub dim: usize,
    pub vectors: Vec<f32>,
    pub chunks: Vec<Chunk>,
}

impl Index {
    // pub async fn update(dir: &str, all_chunks: Vec<Chunk>, model: &GpuBert, tok: &WordPiece, batch: usize) -> Self {
    //     let existing: HashSet<String> = fs::read_dir(dir)
    //         .into_iter().flatten().flatten()
    //         .filter(|e| e.path().is_dir())
    //         .map(|e| e.file_name().to_string_lossy().into_owned())
    //         .collect();
    //
    //     let mut by_vod: HashMap<String, Vec<Chunk>> = HashMap::new();
    //     for c in all_chunks {
    //         if !existing.contains(&c.vod_id) {
    //             by_vod.entry(c.vod_id.clone()).or_default().push(c);
    //         }
    //     }
    //
    //     for (vod_id, chunks) in by_vod {
    //         let embeddings = embed_all_gpu(&model, tok, &chunks, batch);
    //         let sub = Index.
    //     }
    //
    //     todo!()
    // }

    pub fn from_embeddings(chunks: Vec<Chunk>, embeddings: Vec<Vec<f32>>) -> Self {
        assert_eq!(chunks.len(), embeddings.len());
        let dim = embeddings.first().map_or(0, Vec::len);
        let vectors = embeddings.into_iter().flatten().collect();
        Self {
            dim,
            vectors,
            chunks,
        }
    }

    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        min_p: f32,
        dedup_gap: f64,
    ) -> Vec<(f32, &Chunk)> {
        let mut scored: Vec<(f32, &Chunk)> = self
            .vectors
            .chunks_exact(self.dim)
            .zip(&self.chunks)
            .map(|(v, c)| (dot(query, v), c))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // greedy suppression of overlapping results
        let mut kept: Vec<(f32, &Chunk)> = Vec::new();
        for (score, c) in scored {
            let is_dup = kept
                .iter()
                .any(|(_, k)| k.vod_id == c.vod_id && (k.start - c.start).abs() < dedup_gap);
            if !is_dup {
                kept.push((score, c));

                // truncate results to `k`
                if score < min_p || kept.len() == k {
                    break;
                }
            }
        }

        kept
    }

    pub fn save(&self, dir: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut w = BufWriter::new(File::create(format!("{dir}/vectors.f32"))?);

        w.write_all(&(self.dim as u32).to_le_bytes())?;
        for v in &self.vectors {
            w.write_all(&v.to_le_bytes())?;
        }

        w.flush()?;
        let meta = serde_json::to_string(&self.chunks).unwrap();
        fs::write(format!("{dir}/meta.json"), meta)
    }

    pub fn load(dir: &str) -> std::io::Result<Self> {
        let mut bytes = Vec::new();
        File::open(format!("{dir}/vectors.f32"))?.read_to_end(&mut bytes)?;
        tracing::info!("\t- loading precomputed vectors: {dir}/vectors.f32");
        let dim = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        let vectors = bytes[4..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        let chunks = serde_json::from_str(&fs::read_to_string(format!("{dir}/meta.json"))?)?;
        Ok(Self {
            dim,
            vectors,
            chunks,
        })
    }
}

/// Run embedding workload on CUDA cores.
///
/// # Reference
///
/// RTX 4090:
/// ---------
/// NVIDIA CUDA Cores:  16_384
/// Memory Size:        24 GB
///
/// # Tuning params
///
/// > NOTE: I am slowly working through this and implementing some knobs here and there as I go...
///
/// ## @batch
///
/// The `batch` parameter tunes how many chunks we are feeding to the GPU per embed call; higher
/// batch sizes utilize more CUDA cores per `launch` call at the cost of higher memory usage.
///
/// Defaults to 8192, which is a reasonable default for the small (~384-layer) BERT models; larger
/// models will probably OOM on this batch size.
async fn embed_and_save(st: &SafeTensors, tok: &WordPiece, config: Config, batch: usize) -> Index {
    tracing::info!("init GPU model");
    let model = bert::gpu::GpuBert::load(st, config).unwrap();

    tracing::info!("building corpus chunks");
    let chunks = ingest::parse_all("subs").await.unwrap();
    tracing::info!("\t- corpus length: {} chunks", chunks.len());

    let embeddings = embed_all_gpu(&model, tok, &chunks, batch);
    let index = Index::from_embeddings(chunks, embeddings);

    index.save(EMBEDDINGS_OUTPATH).unwrap();
    index
}

#[tracing::instrument(skip_all)]
fn replace_query_nouns(q: &str) -> String {
    q.to_lowercase()
        .replace("walf", "wolf")
        .replace("kerobel", "carobel")
        .replace("tini", "tiny")
        .to_string()
}

#[tokio::main]
async fn main() {
    init_log(Level::TRACE);
    let args = get_args();

    let (model_path, vocab_path, model_config) = get_model_path(args.model);
    let st = SafeTensors::load(&model_path).expect("model file");

    let tok = WordPiece::from_vocab_file(&vocab_path, 256).unwrap();
    let cpu_model = bert::Bert::load(&st, model_config);

    let idx = match args.action {
        args::Action::Save => embed_and_save(&st, &tok, model_config, args.batch).await,
        _ => Index::load(EMBEDDINGS_OUTPATH).unwrap(),
    };

    let query = replace_query_nouns(&args.query);

    let qv = cpu_model.embed(&tok.encode(&query));
    let qr = idx.search(&qv, args.top_k, args.min_p, DEFAULT_DEDUP_GAP_SECS);

    tracing::info!(query, "searching");

    for (i, data) in qr.iter().enumerate() {
        let score = data.0;
        tracing::info!(
            rank = i,
            score = format!("{score:.3}"),
            vod = format!(
                "https://youtube.com/watch?v={}&t={}",
                data.1.vod_id,
                data.1.start.floor() as u32
            ),
        );

        println!("{}\n", data.1.text);
    }
}
