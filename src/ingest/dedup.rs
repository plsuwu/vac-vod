fn norm(tok: &str) -> String {
    tok.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn merge_punct(first: &str, last: &str) -> String {
    let body = first.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '\'');
    let tail_start = last
        .rfind(|c: char| c.is_alphanumeric() || c == '\'')
        .map_or(last.len(), |i| {
            i + last[i..].chars().next().unwrap().len_utf8()
        });

    format!("{body}{}", &last[tail_start..])
}

fn pass(tokens: &[String], max_ngram: usize, keep: usize) -> (Vec<String>, bool) {
    let norms: Vec<String> = tokens.iter().map(|t| norm(t)).collect();
    let mut out = Vec::with_capacity(tokens.len());
    let mut changed = false;
    let mut i = 0;

    while i < tokens.len() {
        let mut consumed = false;
        for n in 1..=max_ngram {
            if i + 2 * n > tokens.len() {
                break;
            }

            if norms[i..i + n].iter().all(|s| s.is_empty()) {
                continue;
            }
            let mut reps = 1;
            while i + (reps + 1) * n <= tokens.len()
                && norms[i..i + n] == norms[i + reps * n..i + (reps + 1) * n]
            {
                reps += 1;
            }

            if reps > keep {
                for r in 0..keep {
                    out.extend(tokens[i + r * n..i + (r + 1) * n].iter().cloned());
                }

                let last_kept = out.len() - 1;
                out[last_kept] = merge_punct(&out[last_kept], &tokens[i + reps * n - 1]);
                i += reps * n;
                changed = true;
                consumed = true;
                break;
            }
        }

        if !consumed {
            out.push(tokens[i].clone());
            i += 1;
        }
    }

    (out, changed)
}

pub fn collapse(text: &str, max_ngram: usize, keep: usize) -> String {
    let keep = keep.max(1);
    let mut tokens: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
    loop {
        let (next, changed) = pass(&tokens, max_ngram, keep);
        if !changed {
            return next.join(" ");
        }
        tokens = next;
    }
}

pub fn merge_overlap(a: &str, b: &str, min_overlap: usize) -> String {
    let at: Vec<&str> = a.split_whitespace().collect();
    let bt: Vec<&str> = b.split_whitespace().collect();
    let an: Vec<String> = at.iter().map(|t| norm(t)).collect();
    let bn: Vec<String> = bt.iter().map(|t| norm(t)).collect();
    for k in (min_overlap..=at.len().min(bt.len())).rev() {
        if an[at.len() - k..] == bn[..k] {
            return at
                .iter()
                .chain(&bt[k..])
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
        }
    }

    format!("{a} {b}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_words_and_phrases() {
        assert_eq!(collapse("I'll I'll I'll wake up", 3, 1), "I'll wake up");
        assert_eq!(
            collapse("No, no, no, no, no. Happy debut.", 3, 1),
            "No. Happy debut."
        );
        assert_eq!(collapse("Yes. Yes. Yes. Yes. Okay.", 3, 1), "Yes. Okay.");
        assert_eq!(collapse("we do we do we do it", 3, 1), "we do it");
        assert_eq!(collapse("a a b a a b", 3, 1), "a b");
        assert_eq!(
            collapse("Maybe we do two streams.", 3, 1),
            "Maybe we do two streams."
        );
    }

    #[test]
    fn keeps_one_echo_when_asked() {
        assert_eq!(collapse("Yes. Yes. Yes. Yes.", 3, 2), "Yes. Yes.");
    }

    #[test]
    fn stitches_overlapping_chunks() {
        let a = "It will either";
        let b = "will either be the Stanley Parable";
        assert_eq!(
            merge_overlap(a, b, 2),
            "It will either be the Stanley Parable"
        );
    }
}
