//! Vite: the app starts at the module scripts of its HTML pages, and its configuration
//! (`vite.config.*`) is tooling that loads Vite's plugins.

use std::path::Path;

use super::{Context, Contribution, Plugin};

pub struct Vite;

impl Plugin for Vite {
    fn name(&self) -> &'static str {
        "vite"
    }

    fn contribute(&self, context: &Context, contribution: &mut Contribution) {
        let config = contribution
            .config_file(context.root, &["vite.config"])
            .unwrap_or_default();
        let vite_root = config["root"].as_str().unwrap_or_default();
        let vite_root = vite_root.trim_start_matches("./").trim_end_matches('/');
        let public = match &config["publicDir"] {
            serde_json::Value::Bool(false) => None,
            dir => Some(dir.as_str().unwrap_or("public")),
        };
        let absolute_roots = public.map(|dir| super::normalize(&Path::new(vite_root).join(dir)));
        contribution.absolute_roots.extend(absolute_roots);
        contribution.absolute_roots.push(vite_root.to_string());
        // `resolve.alias`: an object, or a list of `{ find, replacement }`. A replacement from
        // `/` starts at the Vite root; a relative one, at the configuration.
        let target = |replacement: &str| match replacement.strip_prefix('/') {
            Some(path) => format!("./{}", super::normalize(&Path::new(vite_root).join(path))),
            None => replacement.to_string(),
        };
        match &config["resolve"]["alias"] {
            serde_json::Value::Object(aliases) => {
                for (find, replacement) in aliases {
                    if let Some(replacement) = replacement.as_str() {
                        contribution.alias(find, &target(replacement));
                    }
                }
            }
            serde_json::Value::Array(aliases) => {
                for alias in aliases {
                    if let (Some(find), Some(replacement)) =
                        (alias["find"].as_str(), alias["replacement"].as_str())
                    {
                        contribution.alias(find, &target(replacement));
                    }
                }
            }
            _ => {}
        }
        let pages = ignore::WalkBuilder::new(context.root)
            .require_git(false)
            .filter_entry(|entry| entry.file_name() != "node_modules")
            .build()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|e| e == "html"));
        for page in pages {
            let Ok(html) = std::fs::read_to_string(page.path()) else {
                continue;
            };
            let dir = page
                .path()
                .parent()
                .and_then(|dir| dir.strip_prefix(context.root).ok())
                .unwrap_or(Path::new(""));
            for src in module_scripts(&html) {
                let path = match src.strip_prefix('/') {
                    Some(path) => Path::new(vite_root).join(path),
                    None => dir.join(src),
                };
                let path = super::normalize(&path);
                contribution.production.push(globset::escape(&path));
            }
        }
    }
}

/// The `src` of each `<script type="module">` of a page, except the ones on another host.
fn module_scripts(html: &str) -> Vec<&str> {
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut rest = 0;
    while let Some(start) = lower[rest..].find("<script") {
        let start = rest + start + "<script".len();
        let end = lower[start..]
            .find('>')
            .map_or(html.len(), |end| start + end);
        let attributes = attributes(&html[start..end]);
        let value = |name: &str| {
            attributes
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| *value)
        };
        if value("type").is_some_and(|kind| kind.eq_ignore_ascii_case("module"))
            && let Some(src) = value("src")
            && !src.contains("://")
            && !src.starts_with("//")
        {
            found.push(src);
        }
        rest = end;
    }
    found
}

/// The `name=value` pairs of a tag, with the value quoted or bare.
fn attributes(tag: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = tag.trim_start();
    while let Some(eq) = rest.find('=') {
        let name = rest[..eq].split_whitespace().last().unwrap_or_default();
        let after = rest[eq + 1..].trim_start();
        let (value, next) = match after.chars().next() {
            Some(quote @ ('"' | '\'')) => {
                let body = &after[1..];
                let end = body.find(quote).unwrap_or(body.len());
                (&body[..end], body.get(end + 1..).unwrap_or_default())
            }
            _ => {
                let end = after.find(char::is_whitespace).unwrap_or(after.len());
                (&after[..end], &after[end..])
            }
        };
        found.push((name, value));
        rest = next;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn the_module_scripts_of_each_page_are_production_entries() {
        let dir = project(&[
            (
                "index.html",
                r#"<script type="module" src="/src/main.tsx"></script>
                   <SCRIPT TYPE=module SRC=./src/other.ts></SCRIPT>
                   <script src="/legacy.js"></script>
                   <script type="module">import "./inline.js"</script>
                   <script type='module' src='https://cdn.example.com/x.js'></script>
                   <script type="module" src="//cdn.example.com/y.js"></script>
                   <script type="module" async src="../outside/../src/up.js"/>"#,
            ),
            ("vite.config.ts", "export default { plugins: [] };"),
        ]);
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        assert_eq!(
            found.production,
            strings(&["src/main.tsx", "src/other.ts", "src/up.js"])
        );
        assert_eq!(found.files, strings(&["./vite.config.ts"]));
        assert_eq!(found.absolute_roots, strings(&["public", ""]));
        assert!(found.tests.is_empty() && found.packages.is_empty());
    }

    #[test]
    fn an_absolute_src_starts_at_the_vite_root_and_a_relative_one_at_the_page() {
        let dir = project(&[("vite.config.js", r#"export default { root: "./app/" };"#)]);
        std::fs::create_dir_all(dir.path().join("app/admin")).unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/x")).unwrap();
        std::fs::write(
            dir.path().join("app/admin/index.html"),
            r#"<script type="module" src="/src/admin.ts"></script>
               <script type="module" src="./[id].ts"></script>"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("node_modules/x/index.html"),
            r#"<script type="module" src="/no.js"></script>"#,
        )
        .unwrap();
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        assert_eq!(
            found.production,
            strings(&["app/src/admin.ts", "app/admin/[[]id[]].ts"])
        );
        assert_eq!(found.absolute_roots, strings(&["app/public", "app"]));
        let dir = project(&[(
            "vite.config.js",
            r#"export default { publicDir: "static" };"#,
        )]);
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        assert_eq!(found.absolute_roots, strings(&["static", ""]));
        let dir = project(&[("vite.config.js", "export default { publicDir: false };")]);
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        assert_eq!(found.absolute_roots, strings(&[""]));
    }

    #[test]
    fn resolve_alias_maps_names_from_the_root_or_the_config() {
        let dir = project(&[(
            "vite.config.js",
            r##"export default { root: "app", resolve: { alias: { "@": "/src", "#x": "./lib/x", "n": 1 } } };"##,
        )]);
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        let resolve = |name: &str| found.aliases.iter().find_map(|alias| alias.apply(name));
        assert_eq!(resolve("@/a/b").as_deref(), Some("./app/src/a/b"));
        assert_eq!(resolve("@").as_deref(), Some("./app/src"));
        assert_eq!(resolve("@scope/pkg"), None);
        assert_eq!(resolve("#x").as_deref(), Some("./lib/x"));
        assert_eq!(found.aliases.len(), 2);
        let dir = project(&[(
            "vite.config.js",
            r#"export default { resolve: { alias: [{ find: "~", replacement: "pkg" }, { find: "y" }] } };"#,
        )]);
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        assert_eq!(found.aliases.len(), 1);
        assert_eq!(found.aliases[0].apply("~/z").as_deref(), Some("pkg/z"));
    }

    #[test]
    fn a_page_that_cannot_be_read_and_a_tag_left_open_are_skipped() {
        let dir = project(&[("broken.html", "<script type=module src=/a.js")]);
        std::fs::write(dir.path().join("binary.html"), [0xff, 0xfe]).unwrap();
        let found = setup(dir.path(), &declared(&["vite"]), &json!({}));
        assert_eq!(found.production, strings(&["a.js"]));
        assert!(found.files.is_empty());
    }
}
