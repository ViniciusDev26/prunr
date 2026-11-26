//! React Router in framework mode (`@react-router/dev`) and Remix (`@remix-run/dev`): under the
//! app folder (`app/`, or the config's `appDirectory`), the root route, the route modules, the
//! route config and the client and server entries are production entries. Their exports
//! (`loader`, `action`, `meta`, the component) are the interface the framework reads.
//!
//! The generated types of each route (`./+types/home`) are modules the framework writes; the
//! runtime packages the default entries import (`react-dom`, `isbot`, the server adapter) and
//! Vite, which the dev package requires, are used.

use super::{Context, Contribution, Plugin};

pub struct ReactRouter;

/// The files under the app folder the framework loads by convention.
const APP: &[&str] = &[
    "root.*",
    "routes.*",
    "routes/**",
    "entry.client.*",
    "entry.server.*",
];

/// The packages the default entries and the server import.
const RUNTIME: &[&str] = &[
    "react-dom",
    "isbot",
    "@react-router/node",
    "@react-router/serve",
    "@remix-run/node",
    "@remix-run/serve",
];

/// The packages that enable it.
const DEV: &[&str] = &["@react-router/dev", "@remix-run/dev"];

impl Plugin for ReactRouter {
    fn name(&self) -> &'static str {
        "@react-router/dev"
    }

    fn enabled(&self, context: &Context) -> bool {
        DEV.iter().any(|name| context.declared.contains(*name))
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let config = contribution
            .config_file(context.root, &["react-router.config", "remix.config"])
            .unwrap_or_default();
        let app = config["appDirectory"]
            .as_str()
            .unwrap_or("app")
            .trim_start_matches("./")
            .trim_end_matches('/');
        contribution
            .production
            .extend(APP.iter().map(|glob| format!("{app}/{glob}")));
        contribution
            .production_packages
            .extend(RUNTIME.iter().map(|name| name.to_string()));
        contribution.packages.insert("vite".to_string());
        contribution.alias_regex(r"^\./\+types/", "virtual:react-router");
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn the_app_folder_holds_the_routes_and_their_types_are_generated() {
        let dir = project(&[(
            "remix.config.js",
            r#"module.exports = { appDirectory: "./src/" };"#,
        )]);
        let found = setup(dir.path(), &declared(&["@remix-run/dev"]), &json!({}));
        assert!(found.production.contains(&"src/routes/**".to_string()));
        assert!(found.production.contains(&"src/root.*".to_string()));
        assert_eq!(found.files, strings(&["./remix.config.js"]));
        assert_eq!(
            found.aliases[0].apply("./+types/home").as_deref(),
            Some("virtual:react-router")
        );
        assert_eq!(found.aliases[0].apply("../+types/home"), None);
        let found = setup(dir.path(), &declared(&[]), &json!({}));
        assert!(found.production.is_empty());
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["@react-router/dev"]), &json!({}));
        assert!(found.production.contains(&"app/routes.*".to_string()));
    }
}
