mod bert;
mod ingest;
mod misc;
mod safetensors;
mod tensor;
mod tokenizer;

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};

use tokenizer::wordpiece::WordPiece;

use crate::bert::Config;
use crate::bert::batched::embed_all_gpu;
use crate::ingest::Chunk;
use crate::safetensors::{ST_MODEL_PATH, SafeTensors};
use crate::tensor::dot;

const EMBEDDINGS_OUTPATH: &str = ".embeddings";

pub struct Index {
    pub dim: usize,
    pub vectors: Vec<f32>,
    pub chunks: Vec<Chunk>,
}

impl Index {
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

    pub fn search(&self, query: &[f32], k: usize) -> Vec<(f32, &Chunk)> {
        let mut scored: Vec<(f32, &Chunk)> = self
            .vectors
            .chunks_exact(self.dim)
            .zip(&self.chunks)
            .map(|(v, c)| (dot(query, v), c))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        scored.truncate(k);
        scored
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

async fn embed_and_save(st: &SafeTensors, tok: &WordPiece) -> Index {
    println!("init model");
    let model = bert::gpu::GpuBert::load(st, Config::minilm_l6()).unwrap();

    println!("building corpus chunks");
    let chunks = ingest::parse_all("subs").await.unwrap();
    println!(" --> OK: corpus length: {} chunks", chunks.len());

    let embeddings = embed_all_gpu(&model, tok, &chunks, 128);
    let index = Index::from_embeddings(chunks, embeddings);

    index.save(EMBEDDINGS_OUTPATH).unwrap();
    index
}
//
// async fn load_embeddings() -> Index {}

#[tokio::main]
async fn main() {
    let st = SafeTensors::load(ST_MODEL_PATH).expect("model file");
    let cpu_model = bert::Bert::load(&st, Config::minilm_l6());
    let tok = WordPiece::from_vocab_file(tokenizer::VOCAB_PATH, 256).unwrap();

    // let idx = embed_and_save(&st, &tok).await;
    let idx = Index::load(EMBEDDINGS_OUTPATH).unwrap();

    let query = "vac's favorite part of japan";
    let qv = cpu_model.embed(&tok.encode(query));
    let mut qr = idx.search(&qv, 128);
    qr.reverse();

    println!("matching query: \"{query}\":\n\n");
    let top = &qr[..8];

    for (i, data) in top.iter().enumerate() {
        let score = data.0;
        println!(
            "#{}. score: {score:.4} (https://youtube.com/watch?v={}&t={})",
            i,
            data.1.vod_id,
            data.1.start.floor() as u32
        );
        println!("\n{:?}\n", data.1.text);
    }

    // let qv = cpu_model.embed(&wp.encode(query));
    //
    // let mut scored: Vec<(f32, usize)> = idx.chunks
    //     .iter()
    //     .enumerate()
    //     .map(|(i, v)| (dot(&qv, v), i))
    //     .collect();

    // scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    // scored.reverse();
}
