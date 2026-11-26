//! SvelteKit: the routes (`src/routes/**/+page.svelte`, `+layout`, `+server`...), the hooks,
//! the service worker, the param matchers and the files served as they are (`static/**`) are
//! production entries; `svelte.config.js` is a file of the tool (its adapter and preprocessors
//! are used). `$lib` and the `kit.alias` names are aliases, and `$app`, `$env`,
//! `$service-worker` and the types of each route (`./$types`) are modules the framework
//! provides.

use super::{Context, Contribution, Plugin};

pub struct SvelteKit;

/// The files SvelteKit loads by convention.
const ENTRIES: &[&str] = &[
    "src/routes/**/+*",
    "src/hooks.*",
    "src/service-worker.*",
    "src/service-worker/**",
    "src/params/**",
    "src/instrumentation.server.*",
    "static/**",
];

/// The modules SvelteKit provides, which no package or file holds.
const VIRTUAL: &[&str] = &["$app", "$env", "$service-worker"];

impl Plugin for SvelteKit {
    fn name(&self) -> &'static str {
        "@sveltejs/kit"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let config = contribution
            .config_file(context.root, &["svelte.config"])
            .unwrap_or_default();
        contribution
            .production
            .extend(ENTRIES.iter().map(|glob| glob.to_string()));
        contribution
            .production_packages
            .insert("svelte".to_string());
        contribution.alias("$lib", "./src/lib");
        if let serde_json::Value::Object(aliases) = &config["kit"]["alias"] {
            for (name, path) in aliases {
                if let Some(path) = path.as_str() {
                    contribution.alias(name, &format!("./{}", path.trim_start_matches("./")));
                }
            }
        }
        for name in VIRTUAL {
            contribution.alias(name, "virtual:sveltekit");
        }
        contribution.alias_regex(r"^\./\$types(\.js)?$", "virtual:sveltekit");
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn routes_are_entries_and_its_modules_are_aliases() {
        let dir = project(&[(
            "svelte.config.js",
            r#"export default { kit: { alias: { $ui: "./src/ui", n: 1 } } };"#,
        )]);
        let found = setup(dir.path(), &declared(&["@sveltejs/kit"]), &json!({}));
        let resolve = |name: &str| found.aliases.iter().find_map(|alias| alias.apply(name));
        assert_eq!(resolve("$lib/x").as_deref(), Some("./src/lib/x"));
        assert_eq!(resolve("$ui/button").as_deref(), Some("./src/ui/button"));
        assert_eq!(
            resolve("$app/navigation").as_deref(),
            Some("virtual:sveltekit/navigation")
        );
        assert_eq!(resolve("./$types").as_deref(), Some("virtual:sveltekit"));
        assert_eq!(resolve("./$types.js").as_deref(), Some("virtual:sveltekit"));
        assert_eq!(resolve("../$types"), None);
        assert_eq!(found.files, strings(&["./svelte.config.js"]));
        assert!(found.production.contains(&"src/routes/**/+*".to_string()));
        assert!(found.production_packages.contains("svelte"));
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["@sveltejs/kit"]), &json!({}));
        assert_eq!(found.aliases.len(), 5);
    }
}
