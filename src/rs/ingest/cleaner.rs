// TEST_FILES = ["-3oTZS-brE8.en.vtt", "LWhmb1cbJ6Y.en.vtt", "xZCFvCmz7CA.en.vtt"]
// CUE = re.compile(r"^(\d{2}):(\d{2}):(\d{2})\.(\d{3}) --> ")
// TAGS = re.compile(r"</?c[^>]*>")
// BRACKETED = re.compile(r"\[(music|laughter|__)\]", re.I)
// TS = re.compile(r"<(\d{2}):(\d{2}):(\d{2})\.(\d{3})>")
//

use std::borrow::Cow;

use regex::{Captures, Split, SplitN, regex};
use unicode_normalization::UnicodeNormalization;

use crate::rs::{prelude::Word, util::read_file};

fn strip_c_tags(line: &str) -> Cow<'_, str> {
    regex!(r"</?c[^>]*>").replace_all(line, "")
}

fn find_cues(line: &str) -> Option<Captures<'_>> {
    regex!(r"^(?<h>\d{2}):(?<m>\d{2}):(?<s>\d{2})\.(?<ms>\d{3}) -->").captures(line)
}

fn split_timestamp(line: &str) -> Option<Captures<'_>> {
    regex!(r"<(?<h>\d{2}):(?<m>\d{2}):(?<s>\d{2})\.(?<ms>\d{3})>").captures(line)
}

fn parse_cue(cap: Captures<'_>) -> Option<f64> {
    let h = cap.name("h")?.as_str().parse::<f64>().ok()?;
    let m = cap.name("m")?.as_str().parse::<f64>().ok()?;
    let s = cap.name("s")?.as_str().parse::<f64>().ok()?;
    let ms = cap.name("ms")?.as_str().parse::<f64>().ok()?;

    Some(h * 3600.0 + m * 60.0 + s + ms / 1000.0)
}

fn parse_cue_start(line: &str) -> Option<f64> {
    if let Some(cue) = find_cues(line) {
        parse_cue(cue)
    } else {
        None
    }
}


fn norm(w: &str) -> String {
    let unescaped = html_escape::decode_html_entities(w);
    let normalized: String = unescaped.nfkc().collect();

    normalized.trim().to_string()
}

// def clean_subtitles(filepath: str) -> list[tuple[float, str]]:
//     raw_content = open(filepath).readlines()[4:]
//     content = clean_whitespace(raw_content)
//
//     parsed_cues = []
//     next_start = 0.0
//
//     for line in content:
//         if cue_start := parse_cue_start(line):
//             next_start = cue_start
//
//         else:
//             cue_body = parse_cue_body(line, next_start)
//             if len(cue_body) > 1:
//                 cue_body = collapse_runs(cue_body)
//                 parsed_cues.append(cue_body)
//
//     return list(itertools.chain.from_iterable(parsed_cues))

pub fn clean_subs(raw_content: &str) -> Vec<Word> {
    let mut parsed_cues = Vec::new();
    let mut next_start = 0.0;

    let content = raw_content
        .lines()
        .skip(4)
        .map(|line| line.trim())
        .filter(|line| !line.is_empty());

    for line in content {
        if let Some(cue_start) = parse_cue_start(line) {
            next_start = cue_start;
        } else {
            if let cue_body = parse_cue_body(line, next_start)
                && cue_body.len() > 1
            {
                let cue_body = collapse_runs(&cue_body, 2);
                parsed_cues.push(cue_body)
            }
        }
    }

    tracing::info!("{:#?}", parsed_cues);

    todo!()
}

pub fn parse_cue_body(line: &str, cue_start: f64) -> Vec<Word> {
    let mut words = Vec::new();

    let line = strip_c_tags(line);
    let parts = split_timestamp(&line).into_iter().filter_map(|r| {
        parse_cue(r)
    }).collect::<Vec<_>>();

    tracing::info!(?parts);

    let parts: Vec<String> = Vec::new();

    if let Some(cue_pt) = parts.first().map(|f| f.trim())
        && cue_pt.split(' ').collect::<Vec<_>>().len() == 1
    {
        words.push((cue_start, cue_pt));
    }

    for i in (1..parts.len()).step_by(5) {
        let timestamp = &parts[i..i + 4];
        tracing::info!(?timestamp);

        let text = &parts[i + 4].trim();
    }

    Vec::new()
}
// def parse_cue_body(line: str, cue_start: float) -> list[tuple[float, str]]:
//     line = TAGS.sub("", line)
//     parts = [norm(p) for p in TS.split(line)]
//     words = []
//
//     if cue_pt := parts[0].strip():
//         if len(cue_pt.split(" ")) == 1:
//             words.append((cue_start, cue_pt))
//
//     for i in range(1, len(parts), 5):
//         h, m, s, ms = map(int, parts[i : i + 4])
//         t = h * 3600 + m * 60 + s + ms / 1000
//         text = parts[i + 4].strip()
//
//         words.append((t, text))
//
//         # if len(text) == 1:
//         #     words.append((t, text))
//         # else:
//         #     for word in text:
//         #         words.append((t, word))
//
//     return force_monotonic(words)
//

// def parse_cue_start(line: str) -> float | None:
//     if cue := CUE.findall(line):
//         cue = cue[0]
//         h, m, s, ms = map(int, cue[0:4])
//         t = h * 3600 + m * 60 + s + ms / 1000
//
//         return t
//     return None
//
//
// def clean_whitespace(lines: list[str]) -> list[str]:
//     filtered = list(filter(lambda line: not line.isspace() and line != "", lines))
//
//     return [line.strip() for line in filtered]

// fn strip_whitespace(lines: &[String]) -> Vec<String> {

//
// def norm(w: str) -> str:
//     w = html.unescape(w)
//     w = unicodedata.normalize("NFKC", w)
//
//     return w.strip()
//
//
// def collapse_runs(words: list[tuple[float, str]], max_reps=2):
//     out, run = [], 0
//     for t, w in words:
//         key = w.lower().strip(".,!?")
//         # print(w, "->", key)
//         if out and key == out[-1][1].lower().strip(".,!?"):
//             run += 1
//             if run >= max_reps:
//                 continue
//         else:
//             run = 0
//         out.append((t, w))
//
//     return out
//

fn collapse_runs(words: &[Word], max_reps: usize) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    let mut run = 0;

    for (t, w) in words {
        let key = w.to_lowercase();
        let key = key.trim_matches(|c| c == '.' || c == '?' || c == ',' || c == '!');

        if !out.is_empty()
            && key
                == out[out.len() - 1]
                    .1
                    .to_lowercase()
                    .trim_matches(|c| c == '.' || c == '?' || c == ',' || c == '!')
        {
            run += 1;
            if run >= max_reps {
                continue;
            }
        } else {
            run = 0;
        }

        out.push((*t, w.to_string()));
    }

    out
}

//
// def force_monotonic(words: list[tuple[float, str]]):
//     out, last = [], 0.0
//     for t, w in words:
//         if t < last:
//             t = last
//
//         out.append((t, w))
//         last = t
//     return out
//
fn force_monotonic(words: &mut [Word]) -> Vec<Word> {
    let mut out: Vec<Word> = Vec::new();
    let mut last = 0.0f64;

    for (t, w) in words {
        if *t < last {
            *t = last;
        }

        out.push((*t, w.to_string()));
        last = *t;
    }

    todo!();
}
//
//

#[cfg(test)]
mod test {
    use super::*;
    use crate::rs::prelude::init_stdout_logger;

    #[test]
    fn clean_content_end_to_end() {
        init_stdout_logger();

        let input = r#"WEBVTT
Kind: captions
Language: en

00:04:37.068 --> 00:05:19.590 align:start position:0%
 
[music]

00:05:19.590 --> 00:05:19.600 align:start position:0%
  
Wait.

00:05:31.270 --> 00:05:31.280 align:start position:0%
 
 

00:05:31.280 --> 00:05:32.870 align:start position:0%
 
Guys,<00:05:31.520><c> we're</c><00:05:31.680><c> going</c><00:05:31.759><c> to</c><00:05:31.840><c> be</c><00:05:32.000><c> live</c><00:05:32.240><c> for</c><00:05:32.400><c> like</c><00:05:32.639><c> a</c>
"#;
        clean_subs(input);
    }

    #[test]
    fn strips_c_tags() {
        let input = "long<00:05:33.120><c> time</c><00:05:33.360><c> today.</c>";
        let expected = "long<00:05:33.120> time<00:05:33.360> today.";

        assert_eq!(strip_c_tags(input), expected);
    }

    #[test]
    fn norm_unchanged_plain_string() {
        assert_eq!(norm("hello"), "hello");
    }

    #[test]
    fn norm_trims_whitespace() {
        assert_eq!(norm("\t\nhello\n "), "hello");
        assert_eq!(norm("  hello  "), "hello");
    }

    #[test]
    fn norm_decodes_named_entities() {
        assert_eq!(norm("&amp;&amp; aasdf"), "&& aasdf");
        assert_eq!(norm("a &lt; b &gt; c"), "a < b > c");
    }

    #[test]
    fn norm_decodes_numeric_entities() {
        assert_eq!(norm("&#65;&#66;&#67;"), "ABC");
        assert_eq!(norm("&#x41;&#x42;"), "AB");
    }

    #[test]
    fn norm_retains_empty_string() {
        assert_eq!(norm(""), "");
        assert_eq!(norm("   "), "");
    }

    #[test]
    fn finds_cues() {
        init_stdout_logger();

        let input = "00:04:33.793 --> 00:05:36.550 align:start position:0%";
        let output = parse_cue_start(input);

        tracing::info!("{:?}", output);
    }
}
