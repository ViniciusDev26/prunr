//! NestJS: the entry is the `main` its CLI starts (`nest-cli.json`), end-to-end tests live in
//! `test/`, and it needs packages no file imports.
//!
//! The classes a `@Module` registers are already in the graph: the module imports them.

use serde_json::Value;

use super::{Context, Contribution, Plugin};

pub struct Nest;

impl Plugin for Nest {
    fn name(&self) -> &'static str {
        "@nestjs/core"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let cli = std::fs::read_to_string(context.root.join("nest-cli.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or_default();
        let mut projects = vec![&cli];
        if let Value::Object(more) = &cli["projects"] {
            projects.extend(more.values());
        }
        for project in projects {
            let source = project["sourceRoot"].as_str().unwrap_or("src");
            let entry = project["entryFile"].as_str().unwrap_or("main");
            let glob = format!("{source}/{entry}.*");
            if !contribution.production.contains(&glob) {
                contribution.production.push(glob);
            }
        }
        contribution
            .tests
            .extend(["test/**".into(), "**/*.e2e-spec.*".into()]);
        contribution.packages.insert("@nestjs/cli".into());
        if let Some(collection) = cli["collection"].as_str() {
            contribution.packages.insert(collection.into());
        }
        for package in ["reflect-metadata", "rxjs"] {
            contribution.production_packages.insert(package.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn without_nest_cli_json_the_entry_is_src_main() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["@nestjs/core"]), &json!({}));
        assert_eq!(found.production, strings(&["src/main.*"]));
        assert_eq!(found.tests, strings(&["test/**", "**/*.e2e-spec.*"]));
        assert_eq!(found.packages, declared(&["@nestjs/cli"]));
        assert_eq!(
            found.production_packages,
            declared(&["reflect-metadata", "rxjs"])
        );
    }

    #[test]
    fn nest_cli_json_names_the_entries_of_every_project() {
        let dir = project(&[(
            "nest-cli.json",
            r#"{
                "collection": "@nestjs/schematics",
                "sourceRoot": "apps/api/src",
                "projects": {
                    "api": { "sourceRoot": "apps/api/src" },
                    "worker": { "sourceRoot": "apps/worker/src", "entryFile": "worker" }
                }
            }"#,
        )]);
        let found = setup(dir.path(), &declared(&["@nestjs/core"]), &json!({}));
        assert_eq!(
            found.production,
            strings(&["apps/api/src/main.*", "apps/worker/src/worker.*"])
        );
        assert!(found.packages.contains("@nestjs/schematics"));
    }
}
