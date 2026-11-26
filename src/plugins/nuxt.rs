//! Nuxt: the app's files are found by convention, not imported. Pages, layouts, the plugins and
//! middleware, and what Nuxt auto-imports (components, composables, utils) are production
//! entries, under the source folder (`app/` in Nuxt 4, the root before, or `srcDir`), and so is
//! the server (`server/**`). `nuxt.config.*` is a file of the tool, and its `modules` are used.
//!
//! `~`/`@` name the source folder and `~~`/`@@` the root; `#imports`, `#app`, `#components`...
//! are modules Nuxt generates.

use super::{Context, Contribution, Plugin, names};

pub struct Nuxt;

/// The folders and files under the source folder that Nuxt loads by convention.
const APP: &[&str] = &[
    "app.vue",
    "error.vue",
    "app.config.*",
    "pages/**",
    "layouts/**",
    "components/**",
    "composables/**",
    "utils/**",
    "plugins/**",
    "middleware/**",
];

/// The folders at the root that Nuxt loads by convention.
const ROOT: &[&str] = &["server/**", "modules/**", "shared/**"];

impl Plugin for Nuxt {
    fn name(&self) -> &'static str {
        "nuxt"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let config = contribution
            .config_file(context.root, &["nuxt.config"])
            .unwrap_or_default();
        for module in names(&config["modules"])
            .into_iter()
            .chain(names(&config["buildModules"]))
        {
            contribution.reference(module);
        }
        let nuxt4 =
            context.root.join("app/app.vue").is_file() || context.root.join("app/pages").is_dir();
        let source = match config["srcDir"].as_str() {
            Some(dir) => dir.trim_start_matches("./").trim_end_matches('/'),
            None if nuxt4 => "app",
            None => "",
        };
        let prefix = match source {
            "" => String::new(),
            dir => format!("{dir}/"),
        };
        contribution
            .production
            .extend(APP.iter().map(|glob| format!("{prefix}{glob}")));
        contribution
            .production
            .extend(ROOT.iter().map(|glob| glob.to_string()));
        contribution
            .production_packages
            .extend(["vue".to_string(), "vue-router".to_string()]);
        let source = format!("./{source}");
        for name in ["~", "@"] {
            contribution.alias(name, source.trim_end_matches('/'));
        }
        for name in ["~~", "@@"] {
            contribution.alias(name, ".");
        }
        contribution.alias_regex(
            r"^#(app|imports|components|build|head|internal|vue-router|unhead|app-manifest)(/.*)?$",
            "virtual:nuxt",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn the_source_folder_holds_the_entries_and_the_aliases_point_at_it() {
        let dir = project(&[(
            "nuxt.config.ts",
            r#"export default defineNuxtConfig({ modules: ["@nuxt/ui", ["@pinia/nuxt", {}]], buildModules: ["@nuxtjs/x"] });"#,
        )]);
        std::fs::create_dir_all(dir.path().join("app/pages")).unwrap();
        let found = setup(dir.path(), &declared(&["nuxt"]), &json!({}));
        let resolve = |name: &str| found.aliases.iter().find_map(|alias| alias.apply(name));
        assert_eq!(resolve("~/a").as_deref(), Some("./app/a"));
        assert_eq!(resolve("@/a").as_deref(), Some("./app/a"));
        assert_eq!(resolve("~~/a").as_deref(), Some("./a"));
        assert_eq!(resolve("#imports").as_deref(), Some("virtual:nuxt"));
        assert_eq!(resolve("#app/composables").as_deref(), Some("virtual:nuxt"));
        assert_eq!(resolve("#other"), None);
        assert!(found.production.contains(&"app/pages/**".to_string()));
        assert!(found.production.contains(&"server/**".to_string()));
        for package in ["@nuxt/ui", "@pinia/nuxt", "@nuxtjs/x"] {
            assert!(found.packages.contains(package), "{package}");
        }
    }

    #[test]
    fn nuxt_3_has_its_source_at_the_root_or_in_src_dir() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["nuxt"]), &json!({}));
        assert!(found.production.contains(&"pages/**".to_string()));
        assert_eq!(found.aliases[0].apply("~/a").as_deref(), Some("./a"));
        let dir = project(&[("nuxt.config.js", r#"export default { srcDir: "./src/" };"#)]);
        let found = setup(dir.path(), &declared(&["nuxt"]), &json!({}));
        assert!(found.production.contains(&"src/pages/**".to_string()));
        assert_eq!(found.aliases[0].apply("~/a").as_deref(), Some("./src/a"));
    }
}
