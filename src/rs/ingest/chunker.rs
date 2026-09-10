use std::collections::HashMap;

use serde::Serialize;

use super::super::prelude::Word;

#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub channel_id: String,
    pub video_id: String,
    pub text: String,
    pub start: f64,
    pub end: f64,
}

fn windows(seg: &[Word], size: usize, stride: usize) -> Vec<&[Word]> {
    let mut out = Vec::new();
    if seg.len() <= size {
        out.push(seg);
        return out;
    }

    let mut i = 0;
    while i < seg.len() - size {
        out.push(&seg[i..i + size]);
        i += stride;
    }

    out.push(&seg[seg.len() - size..]);
    out
}

pub fn chunk_parts(
    channel_id: &str,
    video_id: &str,
    words: &[Word],
    size: usize,
    stride: usize,
    max_gap: f64,
) -> Vec<Chunk> {
    if words.is_empty() {
        return Vec::new();
    }

    let mut segments: Vec<Vec<Word>> = Vec::new();
    let mut seg: Vec<Word> = vec![words[0].clone()];

    for pair in words.windows(2) {
        let (prev, cur) = (&pair[0], &pair[1]);
        if cur.0 - prev.0 > max_gap {
            segments.push(std::mem::take(&mut seg));
        }
        seg.push(cur.clone());
    }
    segments.push(seg);

    let mut chunks = Vec::new();
    for seg in &segments {
        for win in windows(seg, size, stride) {
            tracing::info!("{:?}", win);

            chunks.push(Chunk {
                channel_id: channel_id.to_string(),
                video_id: video_id.to_string(),
                start: win[0].0,
                end: win[win.len() - 1].0,
                text: win
                    .iter()
                    .map(|(_, w)| w.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            });
        }
    }

    chunks
}
