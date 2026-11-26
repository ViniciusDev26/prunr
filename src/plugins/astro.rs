//! Astro: the frontmatter (between the `---` lines at the top) and the `<script>` elements of
//! each `.astro` file are a module of the graph, in TypeScript. The pages, the content
//! collections (and their config), the middleware and the actions are production entries, and
//! so is `astro.config.*`, as `astro add` keeps integrations in `dependencies`; and the compiled
//! components import `astro`'s runtime. `astro:content`, `astro:assets`... are URLs of a
//! scheme, which prunr leaves alone.

use super::compile::{Compiled, Compiler, keep, script_blocks};
use super::{Context, Contribution, Plugin};

pub struct Astro;

/// The files Astro loads by convention.
const ENTRIES: &[&str] = &[
    "src/pages/**",
    "src/content/**",
    "src/content.config.*",
    "src/content/config.*",
    "src/middleware.*",
    "src/middleware/index.*",
    "src/actions/index.*",
    "astro.config.*",
];

impl Plugin for Astro {
    fn name(&self) -> &'static str {
        "astro"
    }

    fn contribute(&self, _: &Context, contribution: &mut Contribution) {
        contribution
            .compilers
            .insert("astro".to_string(), Compiler(compile));
        contribution
            .production
            .extend(ENTRIES.iter().map(|glob| glob.to_string()));
        contribution.production_packages.insert("astro".to_string());
    }
}

fn compile(source: &str) -> Compiled {
    let frontmatter =
        regex::Regex::new(r"(?ms)\A\s*---\r?\n(.*?)^---").expect("the pattern compiles");
    let (mut scripts, _) = script_blocks(source);
    scripts.extend(
        frontmatter
            .captures(source)
            .and_then(|captures| captures.get(1))
            .map(|content| content.range()),
    );
    Compiled {
        source: keep(source, &scripts),
        typescript: true,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn an_astro_file_is_its_frontmatter_and_scripts_and_pages_are_entries() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["astro"]), &json!({}));
        let compile = found.compilers["astro"].0;
        let compiled = compile("---\na\n---\n<p>---</p><script>b</script>");
        assert_eq!(
            compiled.source,
            format!("   \na\n   \n{}b{}", " ".repeat(18), " ".repeat(9))
        );
        assert!(compiled.typescript);
        assert_eq!(compile("<p>a</p>").source, "        ");
        assert!(found.production.contains(&"src/content/**".to_string()));
        assert!(found.production.contains(&"astro.config.*".to_string()));
        assert_eq!(found.production_packages, declared(&["astro"]));
    }
}
