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
    let typescript =
        regex::Regex::new(r#"\blang\s*=\s*["']?(ts|typescript)\b"#).expect("the pattern compiles");
    let elements = scripts(source);
    let is_typescript = elements
        .iter()
        .any(|(attributes, _)| typescript.is_match(attributes));
    (
        elements.into_iter().map(|(_, content)| content).collect(),
        is_typescript,
    )
}

/// The `<script>` elements of a markup source, outside HTML comments: their attributes and the
/// range of their contents.
pub fn scripts(source: &str) -> Vec<(&str, Range<usize>)> {
    let element = regex::Regex::new(r"(?s)<!--.*?-->|<script\b([^>]*)>(.*?)</script\s*>")
        .expect("the pattern compiles");
    element
        .captures_iter(source)
        .filter_map(|captures| Some((captures.get(1)?.as_str(), captures.get(2)?.range())))
        .collect()
}
