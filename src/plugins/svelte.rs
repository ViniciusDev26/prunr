//! Svelte: the `<script>` elements of each `.svelte` file are a module of the graph, and the
//! compiled components import `svelte`'s runtime.

use super::compile::{Compiled, Compiler, keep, script_blocks, scripts};
use super::{Context, Contribution, Plugin};

pub struct Svelte;

impl Plugin for Svelte {
    fn name(&self) -> &'static str {
        "svelte"
    }

    fn contribute(&self, _: &Context, contribution: &mut Contribution) {
        contribution
            .compilers
            .insert("svelte".to_string(), Compiler(compile));
        contribution
            .production_packages
            .insert("svelte".to_string());
    }
}

/// The instance script declares the component's props with `export` (`export let label`):
/// those are not exports of the module, so the keyword is blanked, and a list that renames
/// props (`export { b as c }`) with it. The module script
/// (`context="module"`, or `module` in Svelte 5) exports to the importers.
fn compile(source: &str) -> Compiled {
    let (ranges, typescript) = script_blocks(source);
    let mut compiled = keep(source, &ranges).into_bytes();
    let module = regex::Regex::new(r#"\bcontext\s*=\s*["']?module\b|(^|\s)module(\s|=|$)"#)
        .expect("the pattern compiles");
    let export =
        regex::Regex::new(r"\bexport\s*\{[^}]*\}\s*;?|\bexport\s").expect("the pattern compiles");
    for (attributes, content) in scripts(source) {
        if module.is_match(attributes) {
            continue;
        }
        for found in export.find_iter(&source[content.clone()]) {
            let start = content.start + found.start();
            let end = match found.as_str().contains('{') {
                true => content.start + found.end(),
                false => start + "export".len(),
            };
            compiled[start..end]
                .iter_mut()
                .filter(|byte| **byte != b'\n')
                .for_each(|byte| *byte = b' ');
        }
    }
    Compiled {
        source: String::from_utf8(compiled).expect("only ASCII bytes were blanked"),
        typescript,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn a_svelte_file_is_its_scripts_and_svelte_is_its_runtime() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["svelte"]), &json!({}));
        let compiled = found.compilers["svelte"].0("<script context=\"module\">a</script><p/>");
        let instance = found.compilers["svelte"].0(
            "<script module>export const a = 1;</script><script>export let b; export { b as c };</script>",
        );
        let kept: Vec<&str> = instance.source.split_whitespace().collect();
        assert_eq!(kept, ["export", "const", "a", "=", "1;", "let", "b;"]);
        assert_eq!(
            compiled.source,
            format!("{}a{}", " ".repeat(25), " ".repeat(13))
        );
        assert!(!compiled.typescript);
        assert_eq!(found.production_packages, declared(&["svelte"]));
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert!(found.compilers.is_empty());
    }
}
