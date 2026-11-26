//! Svelte: the `<script>` elements of each `.svelte` file are a module of the graph, and the
//! compiled components import `svelte`'s runtime.
//!
//! Off in a SvelteKit project, whose routes are found by convention.

use super::compile::{Compiled, Compiler, keep, script_blocks};
use super::{Context, Contribution, Plugin};

pub struct Svelte;

impl Plugin for Svelte {
    fn name(&self) -> &'static str {
        "svelte"
    }

    fn enabled(&self, context: &Context) -> bool {
        context.declared.contains("svelte") && !context.declared.contains("@sveltejs/kit")
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

fn compile(source: &str) -> Compiled {
    let (scripts, typescript) = script_blocks(source);
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
    fn a_svelte_file_is_its_scripts_and_svelte_is_its_runtime() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["svelte"]), &json!({}));
        let compiled = found.compilers["svelte"].0("<script context=\"module\">a</script><p/>");
        assert_eq!(
            compiled.source,
            format!("{}a{}", " ".repeat(25), " ".repeat(13))
        );
        assert!(!compiled.typescript);
        assert_eq!(found.production_packages, declared(&["svelte"]));
        let found = setup(
            dir.path(),
            &declared(&["svelte", "@sveltejs/kit"]),
            &json!({}),
        );
        assert!(found.compilers.is_empty());
    }
}
