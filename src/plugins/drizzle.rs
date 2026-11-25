//! Drizzle Kit: its configuration (`drizzle.config.*`, or one a script passes with `--config`)
//! is tooling, and the schema files it names are what the migrations are generated from.

use std::path::Path;

use super::{Context, Contribution, Plugin, names};

pub struct DrizzleKit;

impl Plugin for DrizzleKit {
    fn name(&self) -> &'static str {
        "drizzle-kit"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let mut configs = Vec::new();
        configs.extend(contribution.config_file(context.root, &["drizzle.config"]));
        for name in context.configs_passed_to("drizzle-kit") {
            configs.extend(contribution.read(context.root, &name));
        }
        for config in configs {
            for schema in names(&config["schema"]) {
                let path = super::normalize(Path::new(schema));
                let glob = match context.root.join(&path).is_dir() {
                    true => format!("{path}/**"),
                    false => path,
                };
                contribution.production.push(glob);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn the_schema_drizzle_kit_reads_is_an_entry() {
        let dir = project(&[(
            "drizzle.config.ts",
            r#"export default { schema: ["./src/db", "./src/tables/*.ts"] };"#,
        )]);
        std::fs::create_dir_all(dir.path().join("src/db")).unwrap();
        let found = setup(dir.path(), &declared(&["drizzle-kit"]), &json!({}));
        assert_eq!(found.production, strings(&["src/db/**", "src/tables/*.ts"]));
        assert_eq!(found.files, strings(&["./drizzle.config.ts"]));
        let manifest = json!({ "scripts": { "gen": "drizzle-kit generate --config x.json" } });
        let dir = project(&[("x.json", r#"{ "schema": "./schema.ts" }"#)]);
        let found = setup(dir.path(), &declared(&["drizzle-kit"]), &manifest);
        assert_eq!(found.production, strings(&["schema.ts"]));
    }
}
