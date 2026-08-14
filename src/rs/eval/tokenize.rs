//! `\u{2019}` == [Unicode `RIGHT SINGLE QUOTATION MARK (U+2019)`]
//!
//! [Unicode `RIGHT SINGLE QUOTATION MARK (U+2019)`]: https://unicodeplus.com/U+2019

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c == '[' {
            chars.next();
            let mut inner = String::new();
            for c in chars.by_ref() {
                if c == ']' {
                    break;
                }
                inner.push(c);
            }
            let inner = inner.trim().to_lowercase();
            if inner.chars().all(|c| c == '_') && !inner.is_empty() {
                tokens.push("[censored]".to_string());
            } else if !inner.is_empty() {
                tokens.push(format!("[{}]", inner));
            }
        } else if c.is_alphanumeric() {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '\'' || c == '\u{2019}' {
                    word.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let word: String = word
                .trim_matches(|c| c == '\'' || c == '\u{2019}')
                .to_lowercase()
                .replace('\u{2019}', "'");

            if !word.is_empty() {
                tokens.push(word);
            }
        } else {
            chars.next();
        }
    }

    tokens
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn keeps_bracket_tags() {
        let test = "[music] hello [laughter]";
        let expects = vec!["[music]", "hello", "[laughter]"];

        assert_eq!(tokenize(test), expects);
    }

    #[test]
    fn normalizes_censor() {
        let test = "oh [ __ ]";
        let expects = vec!["oh", "[censored]"];

        assert_eq!(tokenize(test), expects);
    }

    #[test]
    fn retains_contractions() {
        let test = "Don't it's hasn't hello";
        let expects = vec!["don't", "it's", "hasn't", "hello"];

        assert_eq!(tokenize(test), expects);
    }

    #[test]
    fn strips_punctuation() {
        let test = "hello, hi! hi... hey?";
        let expects = vec!["hello", "hi", "hi", "hey"];

        assert_eq!(tokenize(test), expects);
    }
}
