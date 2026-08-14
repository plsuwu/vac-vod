use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub channel_id: String,
    pub video_id: String,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalQuery {
    pub query: String,
    pub video_id: String,
    pub time: f64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: usize,
    pub video_id: String,
    pub start: f64,
    pub end: f64,
    pub score: f32,
}

/// Implemented by 'retrieval handlers' - e.g. BM25, embedding, hybrid, ... - so that the eval
/// harness can score them
pub trait Retriever {
    fn search(&self, query: &str, k: usize) -> Vec<SearchResult>;
    fn name(&self) -> &str;
}

pub fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> std::io::Result<Vec<T>> {
    let reader = BufReader::new(File::open(path)?);
    let mut out = Vec::new();

    for (lineno, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let rec: T = serde_json::from_str(&line).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{}:{}: {}", path.display(), lineno + 1, e),
            )
        })?;
        out.push(rec);
    }
    Ok(out)
}

pub fn load_chunks(path: &Path) -> std::io::Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    if path.is_dir() {
        let mut stack = vec![path.to_path_buf()];

        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let p = entry?.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|e| e == "jsonl") {
                    chunks.extend(read_jsonl::<Chunk>(&p)?);
                }
            }
        }
    } else {
        chunks = read_jsonl(path)?;
    }

    Ok(chunks)
}
