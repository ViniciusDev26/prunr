//! Edits to `package.json` that keep the rest of the file as it is: no reformatting, no
//! reordering.

use oxc_span::Span;

use crate::edit::{Edit, apply, remove_items};

/// `text` without `package` in `field` (`dependencies`, `devDependencies`). `None` when it is
/// not there.
pub fn remove(text: &str, field: &str, package: &str) -> Option<String> {
    let object = object_field(text, field)?;
    let index = object.members.iter().position(|m| m.key == package)?;
    Some(apply(text, &object.without(text, index)))
}

/// `text` with `package` moved from `dependencies` to `devDependencies`, in alphabetical order.
/// `None` when it is not in `dependencies`.
pub fn move_to_dev(text: &str, package: &str) -> Option<String> {
    let dependencies = object_field(text, "dependencies")?;
    let index = dependencies.members.iter().position(|m| m.key == package)?;
    let moved = &dependencies.members[index];
    let member = &text[moved.start..moved.end];
    let mut edits = dependencies.without(text, index);
    let insert = |at: usize, text: String| Edit {
        start: at as u32,
        end: at as u32,
        text,
    };
    match object_field(text, "devDependencies") {
        Some(dev) => match dev.members.iter().find(|m| m.key.as_str() > package) {
            _ if dev.members.is_empty() => edits.push(Edit {
                start: dev.open as u32 + 1,
                end: dev.close as u32,
                text: format!(" {member} "),
            }),
            Some(next) => edits.push(insert(
                next.start,
                format!("{member},{}", separator(text, next.start)),
            )),
            None => {
                let last = dev.members.last().expect("not empty");
                let separator = separator(text, last.start);
                edits.push(insert(last.end, format!(",{separator}{member}")));
            }
        },
        None => {
            let indent = match separator(text, dependencies.key) {
                separator if separator.starts_with('\n') => separator[1..].to_string(),
                _ => "  ".to_string(),
            };
            edits.push(insert(
                dependencies.close + 1,
                format!(",\n{indent}\"devDependencies\": {{\n{indent}{indent}{member}\n{indent}}}"),
            ));
        }
    }
    Some(apply(text, &edits))
}

/// What goes between two members: a line break and the indentation of the member at `start`
/// when it starts its own line, a space otherwise.
fn separator(text: &str, start: usize) -> String {
    let line_start = text[..start].rfind('\n').map_or(0, |at| at + 1);
    let indent = &text[line_start..start];
    if line_start > 0 && indent.chars().all(char::is_whitespace) {
        format!("\n{indent}")
    } else {
        " ".to_string()
    }
}

/// One `"key": value` of an object.
struct Member {
    key: String,
    start: usize,
    end: usize,
}

/// An object-valued member of the top-level object.
struct Object {
    /// Where its key starts.
    key: usize,
    open: usize,
    close: usize,
    members: Vec<Member>,
}

impl Object {
    /// The edits that drop member `index`; dropping the only one leaves `{}`.
    fn without(&self, text: &str, index: usize) -> Vec<Edit> {
        if self.members.len() == 1 {
            return vec![Edit::delete(Span::new(
                self.open as u32 + 1,
                self.close as u32,
            ))];
        }
        let items: Vec<Span> = self
            .members
            .iter()
            .map(|m| Span::new(m.start as u32, m.end as u32))
            .collect();
        remove_items(text, &items, &[index])
    }
}

/// The member `name` of the top-level object, when its value is an object.
fn object_field(text: &str, name: &str) -> Option<Object> {
    let bytes = text.as_bytes();
    let open = skip_blank(bytes, 0);
    let (root, _) = members(bytes, open)?;
    let field = root.into_iter().find(|m| m.key == name)?;
    let colon = skip_blank(bytes, key_end(bytes, field.start)?);
    let value = skip_blank(bytes, colon + 1);
    let (members, close) = members(bytes, value)?;
    Some(Object {
        key: field.start,
        open: value,
        close,
        members,
    })
}

fn skip_blank(bytes: &[u8], mut at: usize) -> usize {
    while at < bytes.len() && bytes[at].is_ascii_whitespace() {
        at += 1;
    }
    at
}

/// The offset after the string that starts at `at`.
fn key_end(bytes: &[u8], at: usize) -> Option<usize> {
    if bytes.get(at) != Some(&b'"') {
        return None;
    }
    let mut index = at + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return Some(index + 1),
            _ => index += 1,
        }
    }
    None
}

/// The offset after the value that starts at `at`.
fn value_end(bytes: &[u8], at: usize) -> Option<usize> {
    match bytes.get(at)? {
        b'"' => key_end(bytes, at),
        b'{' => members(bytes, at).map(|(_, close)| close + 1),
        b'[' => {
            let mut index = skip_blank(bytes, at + 1);
            while bytes.get(index)? != &b']' {
                index = skip_blank(bytes, value_end(bytes, index)?);
                if bytes.get(index) == Some(&b',') {
                    index = skip_blank(bytes, index + 1);
                }
            }
            Some(index + 1)
        }
        _ => {
            let end = bytes[at..]
                .iter()
                .position(|b| matches!(b, b',' | b'}' | b']') || b.is_ascii_whitespace())
                .map_or(bytes.len(), |offset| at + offset);
            (end > at).then_some(end)
        }
    }
}

/// The members of the object that opens at `open`, and where it closes.
fn members(bytes: &[u8], open: usize) -> Option<(Vec<Member>, usize)> {
    if bytes.get(open) != Some(&b'{') {
        return None;
    }
    let mut found = Vec::new();
    let mut index = skip_blank(bytes, open + 1);
    while bytes.get(index)? != &b'}' {
        let after_key = key_end(bytes, index)?;
        let key: String = serde_json::from_slice(&bytes[index..after_key]).ok()?;
        let colon = skip_blank(bytes, after_key);
        if bytes.get(colon) != Some(&b':') {
            return None;
        }
        let end = value_end(bytes, skip_blank(bytes, colon + 1))?;
        found.push(Member {
            key,
            start: index,
            end,
        });
        index = skip_blank(bytes, end);
        if bytes.get(index) == Some(&b',') {
            index = skip_blank(bytes, index + 1);
        }
    }
    Some((found, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
  "name": "app",
  "scripts": { "dependencies": "not this one" },
  "dependencies": {
    "a": "1.0.0",
    "b": { "weird": "but valid" },
    "c": "^3"
  },
  "devDependencies": {
    "jest": "29",
    "x\"y": "1"
  }
}
"#;

    #[test]
    fn removing_a_package_keeps_the_rest_of_the_file() {
        assert_eq!(
            remove(MANIFEST, "dependencies", "b").unwrap(),
            MANIFEST.replace("    \"b\": { \"weird\": \"but valid\" },\n", "")
        );
        assert_eq!(
            remove(MANIFEST, "dependencies", "c").unwrap(),
            MANIFEST.replace(",\n    \"c\": \"^3\"", "")
        );
        assert_eq!(
            remove(MANIFEST, "devDependencies", "x\"y").unwrap(),
            MANIFEST.replace(",\n    \"x\\\"y\": \"1\"", "")
        );
    }

    #[test]
    fn removing_the_last_package_leaves_an_empty_object() {
        let text = r#"{ "dependencies" : { "a": "1" }, "b": [1, true, null, -2.5e3] }"#;
        assert_eq!(
            remove(text, "dependencies", "a").unwrap(),
            r#"{ "dependencies" : {}, "b": [1, true, null, -2.5e3] }"#
        );
    }

    #[test]
    fn removing_what_is_not_there_is_nothing() {
        assert_eq!(remove(MANIFEST, "dependencies", "jest"), None);
        assert_eq!(remove(MANIFEST, "peerDependencies", "a"), None);
        assert_eq!(
            remove(r#"{ "dependencies": [] }"#, "dependencies", "a"),
            None
        );
        assert_eq!(remove("{ nope", "dependencies", "a"), None);
        assert_eq!(remove("[]", "dependencies", "a"), None);
    }

    #[test]
    fn moving_to_dev_dependencies_keeps_them_sorted() {
        let moved = move_to_dev(MANIFEST, "a").unwrap();
        assert_eq!(
            moved,
            MANIFEST.replace("    \"a\": \"1.0.0\",\n", "").replace(
                "    \"jest\": \"29\",",
                "    \"a\": \"1.0.0\",\n    \"jest\": \"29\","
            )
        );
        let moved = move_to_dev(MANIFEST, "z").map(|_| ());
        assert_eq!(moved, None);
        let last = move_to_dev(&MANIFEST.replace("\"c\"", "\"zz\""), "zz").unwrap();
        assert!(
            last.contains("    \"x\\\"y\": \"1\",\n    \"zz\": \"^3\"\n  }\n}"),
            "{last}"
        );
    }

    #[test]
    fn moving_creates_dev_dependencies_when_there_are_none() {
        let text = "{\n  \"dependencies\": {\n    \"a\": \"1\"\n  }\n}\n";
        assert_eq!(
            move_to_dev(text, "a").unwrap(),
            "{\n  \"dependencies\": {},\n  \"devDependencies\": {\n    \"a\": \"1\"\n  }\n}\n"
        );
        let text = "{\n  \"dependencies\": { \"a\": \"1\" },\n  \"devDependencies\": {}\n}\n";
        assert_eq!(
            move_to_dev(text, "a").unwrap(),
            "{\n  \"dependencies\": {},\n  \"devDependencies\": { \"a\": \"1\" }\n}\n"
        );
    }
}
