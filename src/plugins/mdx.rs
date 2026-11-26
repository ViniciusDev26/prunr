//! MDX: the `import` and `export` blocks of each `.mdx` file (a paragraph that starts with
//! one, outside code fences) are a module of the graph.
//!
//! On with any of the packages that compile MDX.

use super::compile::{Compiled, Compiler, keep};
use super::{Context, Contribution, Plugin};

pub struct Mdx;

/// The packages that compile MDX for a bundler or framework.
const COMPILERS: &[&str] = &[
    "@mdx-js/mdx",
    "@mdx-js/loader",
    "@mdx-js/rollup",
    "@mdx-js/esbuild",
    "@next/mdx",
    "@astrojs/mdx",
];

impl Plugin for Mdx {
    fn name(&self) -> &'static str {
        "@mdx-js/mdx"
    }

    fn enabled(&self, context: &Context) -> bool {
        COMPILERS
            .iter()
            .any(|name| context.declared.contains(*name))
    }

    fn contribute(&self, _: &Context, contribution: &mut Contribution) {
        contribution
            .compilers
            .insert("mdx".to_string(), Compiler(compile));
    }
}

fn compile(source: &str) -> Compiled {
    let mut ranges = Vec::new();
    let mut fence = false;
    let mut block: Option<usize> = None;
    let mut paragraph_start = true;
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let text = line.trim_end();
        if block.is_some() && text.is_empty() {
            ranges.extend(block.take().map(|start| start..offset));
        }
        if text.starts_with("```") || text.starts_with("~~~") {
            fence = !fence;
        } else if !fence
            && paragraph_start
            && (text.starts_with("import ") || text.starts_with("export "))
        {
            block = Some(offset);
        }
        paragraph_start = text.is_empty();
        offset += line.len();
    }
    ranges.extend(block.map(|start| start..offset));
    Compiled {
        source: keep(source, &ranges),
        typescript: false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn an_mdx_file_is_its_import_and_export_blocks() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["@next/mdx"]), &json!({}));
        let compile = found.compilers["mdx"].0;
        let source = "import a from \"a\"\nexport const b = {\n}\n\n# T import x\n\n```\nimport c from \"c\"\n```\nexport const d = 1";
        let compiled = compile(source);
        let kept: Vec<&str> = compiled
            .source
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        assert_eq!(kept, ["import a from \"a\"", "export const b = {", "}"]);
        assert_eq!(compiled.source.len(), source.len());
        assert!(!compiled.typescript);
        let kept = compile("text\n\nexport const e = 1");
        assert_eq!(kept.source.trim(), "export const e = 1");
        assert!(
            setup(dir.path(), &declared(&[]), &json!({}))
                .compilers
                .is_empty()
        );
    }
}
