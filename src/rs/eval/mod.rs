use std::path::{Path, PathBuf};

use types::{EvalQuery, Retriever};

use crate::rs::eval::{bm25::Bm25Index, types::load_chunks};

pub mod bm25;
pub mod tokenize;
pub mod types;

#[derive(Debug)]
pub struct EvalReport {
    pub retriever: String,
    pub num_queries: usize,
    pub ks: Vec<usize>,
    pub video_recall_at_k: Vec<f64>,
    pub time_hit_at_k: Vec<f64>,
    /// mean reciprocal rank of first time-tolerant hit
    pub mrr: f64,
}

pub struct EvalConfig {
    pub ks: Vec<usize>,
    pub tolerance_s: f64,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            ks: vec![1, 5, 10],
            tolerance_s: 60.0,
        }
    }
}

fn fmt_time(s: f64) -> String {
    let s = s as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

pub fn search(q: &str, k: usize) {
    let path_a = PathBuf::from("./subs/UCaZkRdEEpePJ4EEZznuqh8g/jsonl");
    let path_b = PathBuf::from("./subs/UCBustguC_fsnqDQZxOBgGEg/jsonl");

    let mut chunks = load_chunks(&path_a).unwrap();
    chunks.extend(load_chunks(&path_b).unwrap());

    let t0 = std::time::Instant::now();
    let n = chunks.len();

    let index = Bm25Index::build(chunks);
    tracing::info!("indexed {} chunks (in {:?})", n, t0.elapsed());

    let t1 = std::time::Instant::now();
    let results = index.search(q, k);

    tracing::info!("query in {:?}\n", t1.elapsed());

    for (i, r) in results.iter().take(3).enumerate() {
        let c = index.chunk(r.chunk_id);
        tracing::info!(
            "\n#{:2} [{:6.2}] \n[https://youtube.com/watch?v={}&t={}] -> [{} - {}]",
            i + 1,
            r.score,
            r.video_id,
            r.start as u64,
            fmt_time(r.start),
            fmt_time(r.end),
        );

        tracing::info!(chunk_text = c.text);
    }
}

pub fn eval(retriever: &dyn Retriever, queries: &[EvalQuery], cfg: &EvalConfig) -> EvalReport {
    let max_k = cfg.ks.iter().copied().max().unwrap_or(10);
    let mut video_hits = vec![0usize; cfg.ks.len()];
    let mut time_hits = vec![0usize; cfg.ks.len()];
    let mut rr_sum = 0.0f64;

    for q in queries {
        let results = retriever.search(&q.query, max_k);
        let video_rank = results.iter().position(|r| r.video_id == q.video_id);
        let time_rank = results.iter().position(|r| {
            r.video_id == q.video_id
                && q.time >= r.start - cfg.tolerance_s
                && q.time <= r.end + cfg.tolerance_s
        });

        for (i, &k) in cfg.ks.iter().enumerate() {
            if video_rank.is_some_and(|r| r < k) {
                video_hits[i] += 1;
            }
            if time_rank.is_some_and(|r| r < k) {
                time_hits[i] += 1;
            }
        }

        if let Some(r) = time_rank {
            rr_sum += 1.0 / (r as f64 + 1.0);
        }
    }

    let n = queries.len().max(1) as f64;
    EvalReport {
        retriever: retriever.name().to_string(),
        num_queries: queries.len(),
        ks: cfg.ks.clone(),
        video_recall_at_k: video_hits.iter().map(|&h| h as f64 / n).collect(),
        time_hit_at_k: time_hits.iter().map(|&h| h as f64 / n).collect(),
        mrr: rr_sum / n,
    }
}

pub fn report_misses(retriever: &dyn Retriever, queries: &[EvalQuery], cfg: &EvalConfig) {
    let max_k = cfg.ks.iter().copied().max().unwrap_or(10);
    for q in queries {
        let results = retriever.search(&q.query, max_k);
        let time_hit = results.iter().any(|r| {
            r.video_id == q.video_id
                && q.time >= r.start - cfg.tolerance_s
                && q.time <= r.end + cfg.tolerance_s
        });

        if !time_hit {
            let top = results
                .first()
                .map(|r| format!("{} @ {:.0}s (score {:.2})", r.video_id, r.start, r.score))
                .unwrap_or_else(|| "no results".into());
            tracing::warn!(
                "MISS   {:60}   wanted {} @ {:.0}s, top hit: {}",
                truncate(&q.query, 60),
                q.video_id,
                q.time,
                top
            );
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n - 1).collect::<String>()
    }
}

pub fn print_report(r: &EvalReport) {
    tracing::info!("retriever {}\t queries {}", r.retriever, r.num_queries);
    let mut a = String::new();
    a.push_str(&format!("{:>18}", ""));
    for k in &r.ks {
        a.push_str(&format!("{:>10}", format!("@{}", k)));
    }
    tracing::info!("{a}");

    let mut b = String::new();
    b.push_str(&format!("{:>10}", "video recall"));
    for v in &r.video_recall_at_k {
        b.push_str(&format!("{:>10.3}", v));
    }
    tracing::info!("{b}");

    let mut c = String::new();
    c.push_str(&format!("{:>10}", "time hit"));
    for v in &r.time_hit_at_k {
        c.push_str(&format!("{:>10.3}", v));
    }
    tracing::info!("{c}");

    tracing::info!("{:>18}{:>10.3}", "mrr (time)", r.mrr);
}
