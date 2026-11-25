//! PostCSS: the plugins its configuration (`postcss.config.*`, `.postcssrc*`) names. Bundlers
//! (Next.js, Vite) load the configuration without the project declaring `postcss`, so a
//! configuration file enables the plugin too.
//!
//! The configuration runs at build time, often from `dependencies` in apps built where they are
//! deployed: it is a production entry, and the plugins it names are production packages.

use std::path::Path;

use serde_json::Value;

use super::{CONFIG_EXTENSIONS, Context, Contribution, Plugin, names};

pub struct PostCss;

/// The configuration files PostCSS (through `postcss-load-config`) looks for, in order.
fn candidates() -> Vec<String> {
    let mut candidates = vec![".postcssrc".to_string()];
    for stem in [".postcssrc", "postcss.config"] {
        candidates.extend(
            CONFIG_EXTENSIONS
                .iter()
                .map(|extension| format!("{stem}.{extension}")),
        );
    }
    candidates
}

/// The first configuration file at `root`.
fn config_file(root: &Path) -> Option<String> {
    candidates()
        .into_iter()
        .find(|name| root.join(name).is_file())
}

impl Plugin for PostCss {
    fn name(&self) -> &'static str {
        "postcss"
    }

    fn enabled(&self, context: &Context) -> bool {
        context.declared.contains(self.name()) || config_file(context.root).is_some()
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let Some(name) = config_file(context.root) else {
            return;
        };
        contribution.production.push(globset::escape(&name));
        let source = std::fs::read_to_string(context.root.join(&name)).unwrap_or_default();
        let config = match name.ends_with(".json") || name == ".postcssrc" {
            true => serde_json::from_str(&source).ok(),
            false => super::static_config(&source, Path::new(&name)),
        }
        .unwrap_or_default();
        let plugins = match &config["plugins"] {
            Value::Object(plugins) => plugins.keys().map(String::as_str).collect(),
            plugins => names(plugins),
        };
        let syntaxes = ["parser", "syntax", "stringifier"]
            .into_iter()
            .filter_map(|option| config[option].as_str());
        for package in plugins.into_iter().chain(syntaxes) {
            if !package.starts_with(['.', '/']) {
                let package = crate::resolve::package_name(package).to_string();
                contribution.production_packages.insert(package);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn the_plugins_a_config_names_are_production_packages() {
        let dir = project(&[(
            "postcss.config.mjs",
            r#"export default { plugins: { "@tailwindcss/postcss": {}, "./local.js": {} }, parser: "postcss-scss" };"#,
        )]);
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert_eq!(found.production, strings(&["postcss.config.mjs"]));
        assert_eq!(
            found.production_packages,
            declared(&["@tailwindcss/postcss", "postcss-scss"])
        );
        let dir = project(&[(
            ".postcssrc",
            r#"{ "plugins": ["autoprefixer", ["cssnano", {}]] }"#,
        )]);
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert_eq!(
            found.production_packages,
            declared(&["autoprefixer", "cssnano"])
        );
    }

    #[test]
    fn without_a_config_postcss_adds_nothing() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["postcss"]), &json!({}));
        assert!(found.production.is_empty() && found.production_packages.is_empty());
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert!(found.production.is_empty());
    }
}
