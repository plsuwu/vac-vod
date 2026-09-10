use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::{
    bert::{Bert, gpu::GpuBert},
    ingest::Chunk,
    misc,
    tokenizer::wordpiece::WordPiece,
};

pub fn embed_all(
    model: &Bert,
    tok: &WordPiece,
    texts: &[Chunk],
    tx: UnboundedSender<()>,
) -> Vec<Vec<f32>> {
    let n = texts.len();
    let mut out: Vec<Vec<f32>> = vec![Vec::new(); n];
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map(|t| t.get())
        .unwrap_or(1);

    let slots: Vec<std::sync::Mutex<&mut Vec<f32>>> =
        out.iter_mut().map(std::sync::Mutex::new).collect();

    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let ids = tok.encode(&texts[i].text);
                    let v = model.embed(&ids);

                    _ = tx.send(());

                    **slots[i].lock().unwrap() = v;
                }
            });
        }
    });

    out
}

/// - EncodedCorpus.0 == ordering
/// - EncodedCorpus.1 == encoding
pub type EncodedCorpus = (Vec<usize>, Vec<Vec<u32>>);

pub fn embed_all_gpu(
    model: &GpuBert,
    tok: &WordPiece,
    chunks: &[Chunk],
    batch: usize,
) -> Vec<Vec<f32>> {
    let (tx, rx) = unbounded_channel::<()>();
    let chunks_len = chunks.len();
    tokio::task::spawn(async move { misc::progress(rx, "encode", chunks_len).await });

    let encoded: Vec<Vec<u32>> = chunks
        .iter()
        .map(|c| {
            let t = tok.encode(&c.text);
            _ = tx.send(());
            t
        })
        .collect();

    let (tx2, rx2) = unbounded_channel::<()>();
    let enc_len = encoded.len();
    tokio::task::spawn(async move { misc::progress(rx2, "embed", enc_len).await });

    let mut order: Vec<usize> = (0..encoded.len()).collect();
    order.sort_by_key(|&i| encoded[i].len());
    let mut out = vec![Vec::new(); encoded.len()];

    let tx2_clone = tx2.clone();
    for group in order.chunks(batch) {
        let seqs: Vec<Vec<u32>> = group.iter().map(|&i| encoded[i].clone()).collect();
        let vecs = model.embed_batch(&seqs).expect("model.embed_batch");
        for (&i, v) in group.iter().zip(vecs) {
            out[i] = v;
            _ = tx2_clone.send(());
        }
    }
    out
}

// pub fn unscatter_embedding() {}
// pub fn length_sorted_order(tok: &WordPiece, texts: &[String]) -> Vec<usize> {
//     let mut idx: Vec<usize> = (0..texts.len()).collect();
//     idx.sort_by_key(|&i| tok.encode(&texts[i]).len());
//     idx
// }
