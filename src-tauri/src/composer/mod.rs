use crate::{error::AppError, models::*};
use unicode_segmentation::UnicodeSegmentation;
#[derive(Debug, Clone)]
pub enum NumberPosition {
    Prefix,
    Suffix,
}
#[derive(Debug, Clone)]
pub struct NumberingFormat {
    pub position: NumberPosition,
    pub separator: String,
}
impl Default for NumberingFormat {
    fn default() -> Self {
        Self {
            position: NumberPosition::Prefix,
            separator: " ".into(),
        }
    }
}
fn count(s: &str) -> usize {
    s.graphemes(true).count()
}
fn marker(i: usize, total: usize) -> String {
    format!("{i}/{total}")
}
fn decorate(s: &str, i: usize, total: usize, f: &NumberingFormat) -> String {
    match f.position {
        NumberPosition::Prefix => format!("{}{}{}", marker(i, total), f.separator, s),
        NumberPosition::Suffix => format!("{}{}{}", s, f.separator, marker(i, total)),
    }
}
fn boundary(text: &str, max: usize) -> usize {
    let gs: Vec<(usize, &str)> = text.grapheme_indices(true).collect();
    if gs.len() <= max {
        return text.len();
    }
    let end = gs[max].0;
    let slice = &text[..end];
    if let Some(i) = slice.rfind("\n\n") {
        if i > 0 {
            return i;
        }
    };
    let mut sentence = None;
    for (i, c) in slice.char_indices() {
        if matches!(c, '.' | '!' | '?' | '。' | '！' | '？') {
            sentence = Some(i + c.len_utf8())
        }
    }
    if let Some(i) = sentence {
        if i > 0 {
            return i;
        }
    }
    if let Some(i) = slice.rfind(char::is_whitespace) {
        if i > 0 {
            return i;
        }
    }
    end
}
fn raw_parts(text: &str, max: usize) -> Result<Vec<String>, AppError> {
    if max == 0 {
        return Err(AppError::Validation(
            "destination limit is too small for numbering".into(),
        ));
    }
    let mut rest = text.trim();
    let mut out = vec![];
    while !rest.is_empty() {
        let at = boundary(rest, max);
        out.push(rest[..at].trim().to_owned());
        rest = rest[at..].trim_start()
    }
    Ok(out)
}
pub fn split_thread(
    text: &str,
    limit: usize,
    format: &NumberingFormat,
    force_number: bool,
) -> Result<Vec<String>, AppError> {
    if text.trim().is_empty() {
        return Err(AppError::Validation("post text cannot be empty".into()));
    }
    if count(text) <= limit && !force_number {
        return Ok(vec![text.to_owned()]);
    }
    let mut expected = 2usize;
    for _ in 0..20 {
        let overhead = count(&marker(expected, expected)) + count(&format.separator);
        if overhead >= limit {
            return Err(AppError::Validation(
                "destination limit is too small for numbering".into(),
            ));
        }
        let chunks = raw_parts(text, limit - overhead)?;
        let total = chunks.len();
        if total == expected || marker(total, total).len() == marker(expected, expected).len() {
            return Ok(chunks
                .iter()
                .enumerate()
                .map(|(i, s)| decorate(s, i + 1, total, format))
                .collect());
        }
        expected = total.max(1)
    }
    Err(AppError::Validation("thread could not converge".into()))
}
pub fn preview(post: &CanonicalPost, accounts: &[Account]) -> Result<PublishingPreview, AppError> {
    if post.destination_account_ids.is_empty() {
        return Err(AppError::Validation(
            "select at least one destination".into(),
        ));
    }
    let selected: Vec<&Account> = post
        .destination_account_ids
        .iter()
        .map(|id| {
            accounts
                .iter()
                .find(|a| &a.id == id)
                .ok_or_else(|| AppError::AccountNotFound(id.clone()))
        })
        .collect::<Result<_, _>>()?;
    crate::media::validate(&post.media, &selected)?;
    let min = selected
        .iter()
        .min_by_key(|a| a.capabilities.max_text_length)
        .copied();
    let common = matches!(post.policy, PublishingPolicy::CommonLimit)
        .then(|| min.map(|a| a.capabilities.max_text_length))
        .flatten();
    let destinations = selected
        .iter()
        .map(|a| {
            let limit = common.unwrap_or(a.capabilities.max_text_length);
            let force = matches!(post.policy, PublishingPolicy::AlwaysThread);
            Ok(DestinationPreview {
                account_id: a.id.clone(),
                label: a.instance_url.clone().unwrap_or_else(|| "Bluesky".into()),
                max_length: a.capabilities.max_text_length,
                parts: if post.text.trim().is_empty() && !post.media.is_empty() {
                    vec![String::new()]
                } else {
                    split_thread(&post.text, limit, &NumberingFormat::default(), force)?
                },
            })
        })
        .collect::<Result<_, AppError>>()?;
    Ok(PublishingPreview {
        grapheme_count: count(&post.text),
        effective_limit: common,
        limiting_account_id: common.and_then(|_| min.map(|a| a.id.clone())),
        destinations,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_combining_graphemes() {
        let p = split_thread(
            &"e\u{301}".repeat(12),
            8,
            &NumberingFormat::default(),
            false,
        )
        .expect("split");
        assert!(p.iter().all(|x| count(x) <= 8));
        assert!(p
            .iter()
            .all(|x| !x.contains('\u{301}') || !x.ends_with('e')))
    }
    #[test]
    fn prefers_paragraphs() {
        let p = split_thread(
            "First paragraph.\n\nSecond paragraph is longer.",
            25,
            &NumberingFormat::default(),
            false,
        )
        .expect("split");
        assert!(p[0].contains("First paragraph."));
    }
    #[test]
    fn accounts_for_two_digit_numbering() {
        let p = split_thread(
            &vec!["word"; 40].join(" "),
            20,
            &NumberingFormat::default(),
            false,
        )
        .expect("split");
        assert!(p.len() >= 10);
        assert!(p.iter().all(|x| count(x) <= 20));
        assert!(p
            .last()
            .expect("last")
            .starts_with(&format!("{}/{}", p.len(), p.len())))
    }
    #[test]
    fn suffix_is_configurable() {
        let f = NumberingFormat {
            position: NumberPosition::Suffix,
            separator: " · ".into(),
        };
        let p = split_thread(&"x".repeat(20), 12, &f, false).expect("split");
        assert!(p[0].contains(" · 1/"))
    }
}
