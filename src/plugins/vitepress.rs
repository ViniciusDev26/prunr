//! VitePress: a docs site is its pages and its theme. In each VitePress folder (the one with
//! `.vitepress/`: the root, the folder a script passes to `vitepress dev`, or `docs/`),
//! `.vitepress/theme/index.*` and `.vitepress/config.*` are production entries: the site is
//! what the docs ship, and the config (with the helpers and plugins it imports) builds it.
//!
//! A page is a Vue component: its `<script setup>` imports components, so `.md` files are
//! compiled to their scripts, leaving out what is code in the text (fenced blocks and inline
//! code), and every `.md` of the package is an entry: Markdown is never an unused file, the
//! README outside the site included. The theme's `.vue` components are compiled even where
//! `vue` is not declared: VitePress brings it.

use super::compile::{Compiled, Compiler, keep, script_blocks};
use super::{Context, Contribution, Plugin};

pub struct VitePress;

impl Plugin for VitePress {
    fn name(&self) -> &'static str {
        "vitepress"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let mut dirs = vec![String::new(), "docs".to_string()];
        for arguments in context.arguments("vitepress") {
            // `vitepress dev docs`: the folder follows the command, maybe after flags and their
            // values; only a folder with `.vitepress/` counts.
            let words = arguments
                .iter()
                .skip(1)
                .filter(|word| !word.starts_with('-'));
            dirs.extend(words.map(|dir| dir.trim_end_matches('/').to_string()));
        }
        dirs.sort();
        dirs.dedup();
        let mut sites = 0;
        for dir in dirs {
            let prefix = match dir.trim_start_matches("./") {
                "" => String::new(),
                dir => format!("{dir}/"),
            };
            if !context.root.join(format!("{prefix}.vitepress")).is_dir() {
                continue;
            }
            contribution.production.extend([
                format!("{prefix}.vitepress/theme/index.*"),
                format!("{prefix}.vitepress/config.*"),
            ]);
            sites += 1;
        }
        if sites == 0 {
            return;
        }
        contribution.production.push("**/*.md".to_string());
        contribution
            .compilers
            .insert("md".to_string(), Compiler(markdown));
        contribution
            .compilers
            .insert("vue".to_string(), Compiler(super::vue::compile));
        contribution.production_packages.insert("vue".to_string());
    }
}

/// The `<script>` elements of a page, outside fenced code blocks and inline code.
fn markdown(source: &str) -> Compiled {
    let code =
        regex::Regex::new(r"(?ms)^(```|~~~).*?^(```|~~~)|`[^`\n]*`").expect("the pattern compiles");
    let mut text = source.as_bytes().to_vec();
    for found in code.find_iter(source) {
        text[found.range()]
            .iter_mut()
            .filter(|byte| **byte != b'\n')
            .for_each(|byte| *byte = b' ');
    }
    let text = String::from_utf8(text).expect("only whole characters were blanked");
    let (scripts, typescript) = script_blocks(&text);
    Compiled {
        source: keep(source, &scripts),
        typescript,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn a_page_is_its_scripts_outside_code() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["vitepress"]), &json!({}));
        assert!(found.compilers.is_empty());
        std::fs::create_dir_all(dir.path().join(".vitepress")).unwrap();
        let found = setup(dir.path(), &declared(&["vitepress"]), &json!({}));
        let page = "<script setup lang=\"ts\">a</script>\n```\n<script>b</script>\n```\n`<script>c</script>` é";
        let compiled = found.compilers["md"].0(page);
        assert_eq!(compiled.source.trim(), "a");
        assert_eq!(compiled.source.len(), page.len());
        assert!(compiled.typescript);
        assert!(found.compilers.contains_key("vue"));
        assert_eq!(
            found.production,
            strings(&[".vitepress/theme/index.*", ".vitepress/config.*", "**/*.md"])
        );
    }

    #[test]
    fn each_vitepress_folder_has_a_theme_and_a_config() {
        let dir = project(&[]);
        for folder in ["site/.vitepress", ".vitepress", "docs/.vitepress"] {
            std::fs::create_dir_all(dir.path().join(folder)).unwrap();
        }
        std::fs::write(
            dir.path().join("site/.vitepress/config.ts"),
            "export default {};",
        )
        .unwrap();
        let manifest =
            json!({ "scripts": { "docs": "vitepress dev --port 3000 site/", "x": "vitepress" } });
        let found = setup(dir.path(), &declared(&["vitepress"]), &manifest);
        assert_eq!(
            found.production,
            strings(&[
                ".vitepress/theme/index.*",
                ".vitepress/config.*",
                "docs/.vitepress/theme/index.*",
                "docs/.vitepress/config.*",
                "site/.vitepress/theme/index.*",
                "site/.vitepress/config.*",
                "**/*.md",
            ])
        );
    }
}
