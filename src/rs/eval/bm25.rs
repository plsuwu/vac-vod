use std::collections::HashMap;

use super::tokenize::tokenize;
use super::types::{Chunk, Retriever, SearchResult};

#[derive(Debug, Clone, Copy)]
struct Posting {
    chunk_id: u32,
    tf: u32,
}

pub struct Bm25Index {
    chunks: Vec<Chunk>,
    postings: HashMap<String, Vec<Posting>>,
    doc_len: Vec<u32>,
    avg_doc_len: f32,
    pub k1: f32,
    pub b: f32,
    pub dedupe_window_s: f64,
}

impl Bm25Index {
    pub fn build(chunks: Vec<Chunk>) -> Self {
        let mut postings: HashMap<String, Vec<Posting>> = HashMap::new();
        let mut doc_len = Vec::with_capacity(chunks.len());

        for (chunk_id, chunk) in chunks.iter().enumerate() {
            let tokens = tokenize(&chunk.text);
            doc_len.push(tokens.len() as u32);

            let mut tf: HashMap<&str, u32> = HashMap::new();
            for t in &tokens {
                *tf.entry(t.as_str()).or_insert(0) += 1;
            }

            for (term, count) in tf {
                postings.entry(term.to_string()).or_default().push(Posting {
                    chunk_id: chunk_id as u32,
                    tf: count,
                });
            }
        }

        let total: u64 = doc_len.iter().map(|&l| l as u64).sum();
        let avg_doc_len = total as f32 / doc_len.len().max(1) as f32;

        Bm25Index {
            chunks,
            postings,
            doc_len,
            avg_doc_len,
            k1: 1.2,
            b: 0.75,
            dedupe_window_s: 90.0,
        }
    }

    pub fn num_chunks(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunk(&self, id: usize) -> &Chunk {
        &self.chunks[id]
    }

    /// Lucene-style inverse document frequency/[`tf-idf`] function:
    ///
    /// ```no_run
    /// ln(1 + (N - df + 0.5) / (df + 0.5))
    /// ```
    ///
    /// [`tf-idf`]: https://en.wikipedia.org/wiki/Tf-idf
    fn idf(&self, df: usize) -> f32 {
        let n = self.chunks.len() as f32;
        let df = df as f32;

        (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
    }

    fn score_all(&self, query: &str) -> Vec<(u32, f32)> {
        let mut terms = tokenize(query);
        terms.sort();
        terms.dedup();

        let mut scores: HashMap<u32, f32> = HashMap::new();
        for term in &terms {
            let Some(plist) = self.postings.get(term) else {
                continue;
            };

            let idf = self.idf(plist.len());
            for p in plist {
                let tf = p.tf as f32;
                let len_norm = 1.0 - self.b
                    + self.b * self.doc_len[p.chunk_id as usize] as f32 / self.avg_doc_len;
                let contrib = idf * (tf * (self.k1 + 1.0)) / (tf + self.k1 * len_norm);
                *scores.entry(p.chunk_id).or_insert(0.0) += contrib;
            }
        }

        let mut ranked: Vec<(u32, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
        ranked
    }
}

impl Retriever for Bm25Index {
    fn name(&self) -> &str {
        "bm25"
    }

    fn search(&self, query: &str, k: usize) -> Vec<SearchResult> {
        let ranked = self.score_all(query);
        let mut results: Vec<SearchResult> = Vec::with_capacity(k);

        for (chunk_id, score) in ranked {
            let c = &self.chunks[chunk_id as usize];
            let dup = results.iter().any(|r| {
                r.video_id == c.video_id
                    && c.start < r.end + self.dedupe_window_s
                    && r.start < c.end + self.dedupe_window_s
            });
            if dup {
                continue;
            }

            results.push(SearchResult {
                chunk_id: chunk_id as usize,
                video_id: c.video_id.clone(),
                start: c.start,
                end: c.end,
                score,
            });

            if results.len() == k {
                break;
            }
        }

        results
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init_stdout_logger() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .pretty()
            .init();
    }

    fn mk(video: &str, start: f64, text: &str) -> Chunk {
        Chunk {
            channel_id: "test".into(),
            video_id: video.into(),
            start,
            end: start + 100.0,
            text: text.into(),
        }
    }

    #[test]
    fn rare_terms_outrank_common() {
        let idx = Bm25Index::build(vec![
            mk(
                "a",
                0.0,
                "yahi yeah i actually think that's why uh so many people are getting into football and watching it",
            ),
            mk(
                "b",
                0.0,
                "i was going to say haml you are not hamflow i am penguin hi brad hello hi bat as well hi lurkers",
            ),
            mk(
                "c",
                0.0,
                "let's transform a strike if you what is that if you exhausted a card this turn, gain three energy",
            ),
        ]);

        let res = idx.search("transform three energy", 0);
        assert_eq!(res[0].video_id, "c")
    }

    #[test]
    fn overlapping_is_collapsed() {
        let idx = Bm25Index::build(vec![
            mk(
                "a",
                0.0,
                "mr crawling please protect me should fast go away do you not need don't like don't like don't like i drank somebody's blood does that mean i'm ",
            ),
            mk(
                "b",
                0.0,
                "like tummy tummy is a cute word yummy blood yummy so unfortunately i think we have to change the word tummy to body is what i would say if i was",
            ),
            mk(
                "a",
                50.0,
                "i don't like that maybe we have you don't like ooh person angry mr crawling please protect me should fast go away do you not need don't like",
            ),
        ]);

        let res = idx.search("mr crawling", 10);
        let from_a = res.iter().filter(|r| r.video_id == "a").count();
        assert_eq!(from_a, 1);
    }

    #[test]
    fn distant_hits_are_retained() {
        let idx = Bm25Index::build(vec![
            mk(
                "a",
                0.0,
                "have this omari merch uh on one of my shelves when i drank the omari drink",
            ),
            mk(
                "a",
                5000.0,
                "hey haha remember my stupid fucking omari merch",
            ),
        ]);

        let res = idx.search("omari merch", 10);
        assert_eq!(res.len(), 2);
    }
}
