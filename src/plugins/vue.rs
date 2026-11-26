//! Vue: the `<script>` and `<script setup>` of each `.vue` file are a module of the graph.
//!
//! On in a Nuxt project too, which need not declare `vue`.

use super::compile::{Compiled, Compiler, keep, script_blocks};
use super::{Context, Contribution, Plugin};

pub struct Vue;

impl Plugin for Vue {
    fn name(&self) -> &'static str {
        "vue"
    }

    fn enabled(&self, context: &Context) -> bool {
        context.declared.contains("vue") || context.declared.contains("nuxt")
    }

    fn contribute(&self, _: &Context, contribution: &mut Contribution) {
        contribution
            .compilers
            .insert("vue".to_string(), Compiler(compile));
    }
}

pub(super) fn compile(source: &str) -> Compiled {
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
    fn a_vue_file_is_its_scripts() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["vue"]), &json!({}));
        let compile = found.compilers["vue"].0;
        let compiled = compile(
            "<template><!-- <script>x</script> --></template>\n<script setup lang=\"ts\">let a: é = 1</script>",
        );
        assert_eq!(
            compiled.source,
            format!(
                "{}\n{}let a: é = 1{}",
                " ".repeat(48),
                " ".repeat(24),
                " ".repeat(9)
            )
        );
        assert!(compiled.typescript);
        assert!(!compile("<script>x</script>").typescript);
        let found = setup(dir.path(), &declared(&["nuxt"]), &json!({}));
        assert!(found.compilers.contains_key("vue"));
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert!(found.compilers.is_empty());
    }
}
