//! What the compilers of single-file components share: the module of a file is its scripts,
//! with everything else turned into spaces. Each byte that is not a line break becomes one
//! space, so lines, columns and byte offsets (which `prunr fix` edits by) stay those of the file.

use std::ops::Range;

/// A file the parser cannot read (`.vue`, `.svelte`...) turned into the module it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiled {
    pub source: String,
    /// Whether the scripts are TypeScript (`<script lang="ts">`).
    pub typescript: bool,
}

/// Compiles the source of one kind of file. Plugins register it by extension.
#[derive(Debug, Clone, Copy)]
pub struct Compiler(pub fn(&str) -> Compiled);

impl PartialEq for Compiler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::fn_addr_eq(self.0, other.0)
    }
}

impl Eq for Compiler {}

/// `source` with every byte outside `ranges` blanked, except line breaks.
/// The ranges start and end at characters, so what is kept stays valid UTF-8.
pub fn keep(source: &str, ranges: &[Range<usize>]) -> String {
    let bytes = source
        .bytes()
        .enumerate()
        .map(|(index, byte)| match byte {
            b'\n' => b'\n',
            _ if ranges.iter().any(|range| range.contains(&index)) => byte,
            _ => b' ',
        })
        .collect();
    String::from_utf8(bytes).expect("the ranges are at character boundaries")
}

/// The contents of the `<script>` elements of a markup source (outside HTML comments), and
/// whether one of them is TypeScript.
pub fn script_blocks(source: &str) -> (Vec<Range<usize>>, bool) {
    let element = regex::Regex::new(r"(?s)<!--.*?-->|<script\b([^>]*)>(.*?)</script\s*>")
        .expect("the pattern compiles");
    let typescript =
        regex::Regex::new(r#"\blang\s*=\s*["']?(ts|typescript)\b"#).expect("the pattern compiles");
    let mut ranges = Vec::new();
    let mut is_typescript = false;
    for captures in element.captures_iter(source) {
        if let (Some(attributes), Some(content)) = (captures.get(1), captures.get(2)) {
            is_typescript |= typescript.is_match(attributes.as_str());
            ranges.push(content.range());
        }
    }
    (ranges, is_typescript)
}
