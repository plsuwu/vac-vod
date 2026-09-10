pub mod dedup;
pub mod error;

use std::fs;
use std::path::PathBuf;

use html_escape::decode_html_entities;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::unbounded_channel;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub struct Word {
    /// Subtitle text
    pub text: String,
    /// Word timestamp in VOD (seconds)
    pub t: f64,
}

impl Word {
    pub fn new_decode_ents(w: &str, t: f64) -> Self {
        let decoded = decode_html_entities(w);
        let text: String = decoded.nfkc().collect();

        Self { text, t }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub vod_id: String,
    pub text: String,
    pub start: f64,
    pub end: f64,
}

fn parse_ts(s: &str) -> Option<f64> {
    let (hms, ms) = s.trim().split_once('.')?;
    let ms: f64 = ms.parse().ok()?;
    let mut secs = 0.0;
    for part in hms.split(':') {
        secs = secs * 60.0 + part.parse::<f64>().ok()?;
    }

    Some(secs + ms / 1000.0)
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }

    out
}

fn flush(buf: &mut String, t: f64, out: &mut Vec<Word>) {
    for w in buf.split_whitespace() {
        out.push(Word::new_decode_ents(w, t));
    }

    buf.clear();
}

fn extract_words(line: &str, cue_start: f64, out: &mut Vec<Word>) {
    let line = line.replace("[&nbsp;__&nbsp;]", "[__]");
    let mut t = cue_start;
    let mut strbuf = String::new();
    let mut chars = line.chars();

    // let flush = |buf: &mut String, t: f64, out: &mut Vec<Word>| {};

    while let Some(c) = chars.next() {
        if c == '<' {
            let tag: String = chars.by_ref().take_while(|&c| c != '>').collect();
            if let Some(ts) = parse_ts(&tag) {
                flush(&mut strbuf, t, out);
                t = ts;
            }
            // drops e.g. <c> </c> <b> <i> ...
        } else {
            strbuf.push(c);
        }
    }

    flush(&mut strbuf, t, out);
}

#[inline]
fn maybe_ti(lines: &[&str]) -> Option<usize> {
    lines.iter().position(|l| l.contains("-->"))
}

#[inline]
fn maybe_start(lines: &[&str], ti: usize) -> Option<f64> {
    lines[ti].split("-->").next().and_then(parse_ts)
}

pub fn parse_str(src: &str) -> Vec<Word> {
    let src = src.replace("\r\n", "\n");
    let mut words: Vec<Word> = Vec::new();
    let mut prev_plain = String::new();

    for block in src.split("\n\n") {
        if block.is_empty() {
            continue;
        }
        let lines: Vec<&str> = block.lines().collect();

        let Some(ti) = maybe_ti(&lines) else {
            continue;
        };
        let Some(start) = maybe_start(&lines, ti) else {
            continue;
        };

        let payload = &lines[ti + 1..];
        if payload.is_empty() {
            continue;
        }

        let skip_first = payload.len() >= 2 && strip_tags(payload[0]).trim() == prev_plain;
        for (i, line) in payload.iter().enumerate() {
            if i == 0 && skip_first {
                continue;
            }

            if line.trim().is_empty() {
                continue;
            }

            extract_words(line, start, &mut words);
            prev_plain = strip_tags(line).trim().to_string();
        }
    }

    words
}

pub fn chunk(words: &[Word], vod_id: &str, win: f64, stride: f64) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    if words.is_empty() {
        return chunks;
    }
    let mut win_start = words[0].t;
    let last_t = words.last().unwrap().t; // this shouldn't error (?)
    let mut lo = 0;
    while win_start <= last_t {
        while lo < words.len() && words[lo].t < win_start {
            lo += 1
        }

        let mut hi = lo;
        while hi < words.len() && words[hi].t < win_start + win {
            hi += 1
        }

        if hi > lo {
            let text = words[lo..hi]
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            chunks.push(Chunk {
                text,
                vod_id: vod_id.to_string(),
                start: words[lo].t,
                end: words[hi - 1].t,
            });
        }
        win_start += stride;
    }
    chunks
}

pub fn parse_one(filepath: &PathBuf) -> error::Result<Vec<Chunk>> {
    let content = fs::read_to_string(filepath)?;
    let vod_id = {
        let filename = filepath.iter().next_back().unwrap().to_string_lossy();
        String::from(filename.split('.').next().unwrap())
    };

    let parsed = parse_str(&content);
    let chunked = chunk(&parsed, &vod_id, 30.0, 15.0);

    let chunked = chunked
        .into_iter()
        .map(|mut c| {
            c.text = dedup::collapse(&c.text.replace(['\t', '\n'], " "), 3, 1);
            c
        })
        .collect();

    Ok(chunked)
}

pub async fn parse_all(subs_dir: &str) -> error::Result<Vec<Chunk>> {
    let (tx, rx) = unbounded_channel::<()>();

    let mut output = Vec::new();
    let base_dir = PathBuf::from(subs_dir).canonicalize()?;
    let sub_filepaths = ingest(base_dir)?;
    let num_files = sub_filepaths.len();

    tokio::task::spawn(async move { crate::misc::progress(rx, "corpus", num_files).await });

    for filepath in sub_filepaths {
        let parsed = parse_one(&filepath)?;
        output.extend(parsed);
        _ = tx.send(());
    }

    Ok(output)
}

fn ingest(base_dir: PathBuf) -> error::Result<Vec<PathBuf>> {
    // let mut sub_tracks: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut sub_track_files: Vec<PathBuf> = Vec::new();
    let channel_dirs = std::fs::read_dir(base_dir)?.collect::<Vec<_>>();

    for channel_dir in channel_dirs.into_iter() {
        let dir = match channel_dir {
            Ok(de) => de,
            Err(e) => return Err(error::ParserError::IoErr(e)),
        };

        if dir.file_type()?.is_dir() {
            let id = dir.file_name();
            let id = id.to_string_lossy();

            println!("assuming channel id: {}", id);

            let subs_path = dir.path().join("raw");
            let vtts = fs::read_dir(subs_path)?
                .filter_map(|f| f.map(|v| v.path()).ok())
                .collect::<Vec<_>>();

            // // TODO: if we end up caring about multiple channels, we probably
            // // want to do something like this...
            // // --------------------------------------------------------------
            // let entry = sub_tracks
            //     .entry(id.to_string())
            //     .and_modify(|f| f.extend(vtts.clone()))
            //     .or_insert(vtts);

            sub_track_files.extend(vtts);
        }
    }

    Ok(sub_track_files)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub channel_id: String,
    pub parsed_files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParserManifest {
    pub entries: Vec<ManifestEntry>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn word_ctor_correctly_decodes_entities() {
        let w = "&nbsp;&amp;&lt;&gt;&quot;&#39;";
        let t = f64::default();

        let res = Word::new_decode_ents(w, t);
        assert_eq!(&res.text, " &<>\"'");
    }
}
