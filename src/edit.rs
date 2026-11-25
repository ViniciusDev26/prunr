//! Edits to source text: byte ranges replaced by new text, applied from the end so earlier
//! offsets stay valid.

use oxc_span::Span;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edit {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

impl Edit {
    pub fn delete(span: Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
            text: String::new(),
        }
    }
}

/// `source` with `edits` applied. An edit that overlaps one already applied is skipped: it is
/// left for the next round.
pub fn apply(source: &str, edits: &[Edit]) -> String {
    // An edit inside a larger one is part of it.
    let mut edits: Vec<Edit> = edits
        .iter()
        .filter(|edit| {
            !edits.iter().any(|other| {
                other.start <= edit.start
                    && edit.end <= other.end
                    && (other.end - other.start) > (edit.end - edit.start)
            })
        })
        .cloned()
        .collect();
    edits.sort_by(|a, b| b.cmp(a));
    let mut text = source.to_string();
    let mut limit = source.len() as u32;
    for edit in edits {
        if edit.end > limit {
            continue;
        }
        text.replace_range(edit.start as usize..edit.end as usize, &edit.text);
        limit = edit.start;
    }
    text
}

/// Deleting `span` as whole lines: with the indentation before it and the line break after it,
/// when nothing else is on those lines.
pub fn delete_lines(source: &str, span: Span) -> Edit {
    let bytes = source.as_bytes();
    let blank = |b: &u8| *b == b' ' || *b == b'\t';
    let mut start = span.start as usize;
    while start > 0 && blank(&bytes[start - 1]) {
        start -= 1;
    }
    let mut end = span.end as usize;
    while end < bytes.len() && blank(&bytes[end]) {
        end += 1;
    }
    let line_start = start == 0 || bytes[start - 1] == b'\n';
    let rest = &source[end..];
    let line_end = ["\r\n", "\n", ""]
        .into_iter()
        .find(|ending| rest.starts_with(ending) && (!ending.is_empty() || rest.is_empty()));
    match line_end {
        Some(ending) if line_start => {
            Edit::delete(Span::new(start as u32, (end + ending.len()) as u32))
        }
        _ => Edit::delete(span),
    }
}

/// The edits that drop the items at `removed` from a comma-separated list whose items are at
/// `items`, keeping the commas between the rest right.
pub fn remove_items(source: &str, items: &[Span], removed: &[usize]) -> Vec<Edit> {
    let kept = |index: usize| index < items.len() && !removed.contains(&index);
    let mut edits = Vec::new();
    let mut index = 0;
    while index < items.len() {
        if kept(index) {
            index += 1;
            continue;
        }
        let first = index;
        while index < items.len() && !kept(index) {
            index += 1;
        }
        let last = index - 1;
        let span = if kept(index) {
            Span::new(items[first].start, items[index].start)
        } else if first > 0 {
            Span::new(items[first - 1].end, items[last].end)
        } else {
            Span::new(items[first].start, after_comma(source, items[last].end))
        };
        edits.push(Edit::delete(span));
    }
    edits
}

/// The offset after the comma that follows `end`, if one does.
fn after_comma(source: &str, end: u32) -> u32 {
    let rest = &source[end as usize..];
    let trimmed = rest.trim_start();
    match trimmed.strip_prefix(',') {
        Some(after) => (source.len() - after.len()) as u32,
        None => end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(start: u32, end: u32, text: &str) -> Edit {
        Edit {
            start,
            end,
            text: text.into(),
        }
    }

    #[test]
    fn edits_apply_from_the_end() {
        assert_eq!(
            apply(
                "abcdef",
                &[edit(0, 1, "X"), edit(4, 6, ""), edit(2, 2, "-")]
            ),
            "Xb-cd"
        );
    }

    #[test]
    fn an_edit_that_overlaps_another_is_skipped() {
        assert_eq!(apply("abcdef", &[edit(1, 4, ""), edit(2, 5, "Z")]), "abZf");
        assert_eq!(apply("abcdef", &[edit(1, 3, ""), edit(1, 3, "")]), "adef");
    }

    #[test]
    fn deleting_whole_lines_takes_their_indentation_and_line_break() {
        let source = "a;\n  const x = 1;\nb;";
        let span = Span::new(5, 17);
        assert_eq!(&source[5..17], "const x = 1;");
        assert_eq!(apply(source, &[delete_lines(source, span)]), "a;\nb;");
        let source = "a;\nconst x = 1;  \nb;";
        assert_eq!(
            apply(source, &[delete_lines(source, Span::new(3, 15))]),
            "a;\nb;"
        );
        let source = "a; const x = 1;\nb;";
        let span = Span::new(3, 15);
        assert_eq!(apply(source, &[delete_lines(source, span)]), "a; \nb;");
        let source = "const x = 1;";
        assert_eq!(apply(source, &[delete_lines(source, Span::new(0, 12))]), "");
        let source = "a;\r\nconst x = 1;\r\nb;";
        assert_eq!(
            apply(source, &[delete_lines(source, Span::new(4, 16))]),
            "a;\r\nb;"
        );
    }

    fn spans(source: &str, items: &[&str]) -> Vec<Span> {
        items
            .iter()
            .map(|item| {
                let start = source.find(item).unwrap() as u32;
                Span::new(start, start + item.len() as u32)
            })
            .collect()
    }

    fn without(source: &str, items: &[&str], removed: &[usize]) -> String {
        let spans = spans(source, items);
        apply(source, &remove_items(source, &spans, removed))
    }

    #[test]
    fn removing_list_items_keeps_the_commas_right() {
        let source = "{ a, b, c }";
        let items = ["a", "b", "c"];
        assert_eq!(without(source, &items, &[0]), "{ b, c }");
        assert_eq!(without(source, &items, &[1]), "{ a, c }");
        assert_eq!(without(source, &items, &[2]), "{ a, b }");
        assert_eq!(without(source, &items, &[0, 1]), "{ c }");
        assert_eq!(without(source, &items, &[0, 2]), "{ b }");
        assert_eq!(without(source, &items, &[0, 1, 2]), "{  }");
    }

    #[test]
    fn removing_list_items_handles_one_item_per_line_and_trailing_commas() {
        let source = "{\n  a,\n  b,\n  c,\n}";
        let items = ["a", "b", "c"];
        assert_eq!(without(source, &items, &[1]), "{\n  a,\n  c,\n}");
        assert_eq!(without(source, &items, &[2]), "{\n  a,\n  b,\n}");
        assert_eq!(without(source, &items, &[0, 1, 2]), "{\n  \n}");
    }
}
