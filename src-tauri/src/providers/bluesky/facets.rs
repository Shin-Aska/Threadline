use crate::error::AppError;
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

static PATTERNS: LazyLock<Result<(Regex, Regex, Regex), regex::Error>> = LazyLock::new(|| {
    Ok((
        Regex::new(r"(?:^|\s)([#＃])([^\s\x{00ad}\x{2060}\x{200a}-\x{200d}\x{20e2}]+)")?,
        Regex::new(r"\p{P}+$")?,
        Regex::new(r"[^\d\s\p{P}]")?,
    ))
});
#[derive(Debug, Serialize)]
pub struct Facet {
    index: ByteRange,
    features: Vec<Tag>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ByteRange {
    byte_start: usize,
    byte_end: usize,
}
#[derive(Debug, Serialize)]
struct Tag {
    #[serde(rename = "$type")]
    kind: &'static str,
    tag: String,
}
pub fn hashtags(text: &str) -> Result<Vec<Facet>, AppError> {
    let (pattern, punctuation, meaningful) = PATTERNS
        .as_ref()
        .map_err(|error| AppError::Provider(format!("Invalid hashtag pattern: {error}")))?;
    let mut facets = Vec::new();
    for found in pattern.captures_iter(text) {
        let (Some(hash), Some(value)) = (found.get(1), found.get(2)) else {
            continue;
        };
        let tag = punctuation.replace(value.as_str(), "");
        if tag.starts_with('\u{fe0f}')
            || !meaningful.is_match(&tag)
            || tag.graphemes(true).count() > 64
            || tag.len() > 640
        {
            continue;
        }
        facets.push(Facet {
            index: ByteRange {
                byte_start: hash.start(),
                byte_end: value.start() + tag.len(),
            },
            features: vec![Tag {
                kind: "app.bsky.richtext.facet#tag",
                tag: tag.into_owned(),
            }],
        });
    }
    Ok(facets)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashtag_facets_use_utf8_offsets_after_unicode_and_numbering() {
        let text = "1/2 🧵 café #Rust #日本語!";
        let facets = hashtags(text).expect("facets");
        assert_eq!(facets.len(), 2);
        for (facet, expected) in facets.iter().zip(["#Rust", "#日本語"]) {
            assert_eq!(
                &text[facet.index.byte_start..facet.index.byte_end],
                expected
            );
            assert_eq!(facet.features[0].tag, expected.trim_start_matches('#'));
        }
    }
    #[test]
    fn hashtag_detection_ignores_urls_numbers_and_overlong_tags() {
        let text = format!(
            "https://example.test/#fragment word#inside #123 #{} ＃café,",
            "a".repeat(65)
        );
        let facets = hashtags(&text).expect("facets");
        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].features[0].tag, "café");
        assert_eq!(
            &text[facets[0].index.byte_start..facets[0].index.byte_end],
            "＃café"
        );
    }
}
