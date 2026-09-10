use std::{collections::HashMap, fs};

use crate::tokenizer::error::Result;

pub struct WordPiece {
    vocab: HashMap<String, u32>,
    pub cls: u32,
    pub sep: u32,
    pub pad: u32,
    pub unk: u32,
    max_len: usize,
    max_chars_per_word: usize,
    id_to_tok: Vec<String>,
}

impl WordPiece {
    pub fn from_vocab_file(path: &str, max_len: usize) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let id_to_tok = text.lines();
        let vocab: HashMap<String, u32> = id_to_tok
            .clone()
            .enumerate()
            .map(|(i, tok)| (tok.trim_end().to_string(), i as u32))
            .collect();

        let get = |s: &str| {
            *vocab
                .get(s)
                .unwrap_or_else(|| panic!("missing token for {s}"))
        };

        Ok(Self {
            id_to_tok: id_to_tok.map(str::to_owned).collect(),
            max_len,
            cls: get("[CLS]"),
            sep: get("[SEP]"),
            pad: get("[PAD]"),
            unk: get("[UNK]"),
            vocab: vocab.clone(),
            max_chars_per_word: 100,
        })
    }

    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut ids = vec![self.cls];
        for word in naive_tokenize(text) {
            if ids.len() >= self.max_len - 1 {
                break;
            }
            ids.extend(self.wordpiece(&word));
        }
        ids.truncate(self.max_len - 1);
        ids.push(self.sep);
        ids
    }

    fn wordpiece(&self, word: &str) -> Vec<u32> {
        let chars: Vec<char> = word.chars().collect();
        if chars.len() > self.max_chars_per_word {
            return vec![self.unk];
        }
        let mut out = Vec::new();
        let mut start = 0;
        while start < chars.len() {
            let mut end = chars.len();
            let mut found = None;

            while start < end {
                let mut piece: String = chars[start..end].iter().collect();
                if start > 0 {
                    piece.insert_str(0, "##");
                }
                if let Some(&id) = self.vocab.get(&piece) {
                    found = Some(id);
                    break;
                }
                end -= 1;
            }
            match found {
                Some(id) => {
                    out.push(id);
                    start = end;
                }
                None => return vec![self.unk],
            }
        }
        out
    }

    pub fn debug_decode(&self, ids: &[u32]) -> Vec<&str> {
        ids.iter()
            .map(|id| self.id_to_tok[*id as usize].as_str())
            .collect()
    }
}

fn flush(cur: &mut String, toks: &mut Vec<String>) {
    if !cur.is_empty() {
        toks.push(std::mem::take(cur));
    }
}

fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation() || (!c.is_alphanumeric() && !c.is_whitespace())
}

fn naive_tokenize(text: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();

    for c in text.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_whitespace() || c.is_control() {
            flush(&mut cur, &mut toks);
        } else if is_punct(c) {
            flush(&mut cur, &mut toks);
            toks.push(c.to_string());
        } else {
            cur.push(c);
        }
    }

    flush(&mut cur, &mut toks);
    toks
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tokenizes_whole_words() {
        let wp = WordPiece::from_vocab_file(super::super::VOCAB_PATH, 256).unwrap();
        let words = "hello hi foo bar";
        let expected_tokens = [101, 7592, 7632, 29379, 3347, 102];

        let out = wp.encode(words);
        for (n, t) in out.iter().enumerate() {
            assert_eq!(t, &expected_tokens[n]);
        }
    }
}
