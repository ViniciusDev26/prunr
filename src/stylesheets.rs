//! Stylesheets: the packages a CSS, Sass or Less file uses through `@import`, `@use`,
//! `@forward`, `@plugin`, `@reference` and `@config`, and through the local stylesheets it
//! imports. A JavaScript file that imports a stylesheet uses those packages.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The extensions of the stylesheets prunr reads.
const EXTENSIONS: &[&str] = &["css", "scss", "sass", "less", "pcss", "postcss"];

/// The at-rules that load another stylesheet, a plugin or a configuration.
const RULES: &[&str] = &["import", "use", "forward", "plugin", "reference", "config"];

/// Whether `path` is a stylesheet.
pub fn is_stylesheet(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| EXTENSIONS.contains(&extension))
}

/// The packages the stylesheet at `path` uses, and those the local stylesheets it imports use.
pub fn packages(path: &Path) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut pending = vec![path.to_path_buf()];
    while let Some(path) = pending.pop() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap_or_default();
        let dir = path.parent().unwrap_or(Path::new(""));
        for specifier in specifiers(&source) {
            if specifier.contains("://") || specifier.starts_with("//") || specifier.contains(':') {
                continue;
            }
            let specifier = specifier.trim_start_matches('~');
            match local(dir, specifier) {
                Some(file) if is_stylesheet(&file) => pending.push(file),
                Some(_) => {}
                None if specifier.starts_with(['.', '/']) => {}
                None => {
                    packages.insert(crate::resolve::package_name(specifier).to_string());
                }
            }
        }
    }
    packages
}

/// The file a specifier names next to the stylesheet, as Sass and `postcss-import` look for it:
/// as is, with a stylesheet extension, or as a Sass partial (`_name.scss`).
fn local(dir: &Path, specifier: &str) -> Option<PathBuf> {
    let path = dir.join(specifier);
    let name = path.file_name()?.to_string_lossy().to_string();
    let partial = path.with_file_name(format!("_{name}"));
    let with = |path: &PathBuf, extension: &&str| {
        let mut name = path.clone().into_os_string();
        name.push(format!(".{extension}"));
        PathBuf::from(name)
    };
    std::iter::once(path.clone())
        .chain(EXTENSIONS.iter().map(|extension| with(&path, extension)))
        .chain(EXTENSIONS.iter().map(|extension| with(&partial, extension)))
        .find(|path| path.is_file())
}

/// The quoted (or `url(...)`) argument of each at-rule that loads something, outside comments.
fn specifiers(source: &str) -> Vec<String> {
    let mut code = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("/*") {
        code.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .split_once("*/")
            .map_or("", |(_, after)| after);
    }
    code.push_str(rest);
    let mut found = Vec::new();
    for (at, _) in code.match_indices('@') {
        let after = &code[at + 1..];
        let Some(rule) = RULES.iter().find(|rule| {
            after.starts_with(*rule) && after[rule.len()..].starts_with(|c: char| c.is_whitespace())
        }) else {
            continue;
        };
        let mut argument = after[rule.len()..].trim_start();
        if let Some(inside) = argument.strip_prefix("url(") {
            argument = inside.trim_start();
        }
        let value = match argument.chars().next() {
            Some(quote @ ('"' | '\'')) => argument[1..].split(quote).next(),
            _ => argument.split([')', ';', ' ', '\n']).next(),
        };
        found.extend(value.filter(|value| !value.is_empty()).map(str::to_string));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_at_rules_that_load_something_name_it() {
        assert_eq!(
            specifiers(
                "@import \"a\";\n@import url('b') layer(x);\n@import url(c);\n@use 'd' as *;\n\
                 /* @import \"gone\"; */ @plugin \"e\"; @reference \"f\"; @config './g.js';\n\
                 @forward \"h\"; @important; @media print {} @import ;"
            ),
            ["a", "b", "c", "d", "e", "f", "./g.js", "h"]
        );
        assert_eq!(
            specifiers("/* never closed @import \"x\";"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_stylesheet_uses_the_packages_it_and_its_local_imports_name() {
        let dir = tempfile::tempdir().unwrap();
        for (name, content) in [
            (
                "app.css",
                "@import \"tailwindcss\"; @import \"./theme\"; @import \"https://x.com/a.css\"; @import \"./gone.css\"; @config \"./tw.js\"; @import \"local\";",
            ),
            (
                "theme.css",
                "@import \"@fontsource/inter/400.css\"; @import \"app.css\";",
            ),
            ("local.css", ""),
            ("tw.js", ""),
            (
                "x.scss",
                "@use \"sass:math\"; @use \"~bootstrap/scss/x\"; @forward \"mixins\";",
            ),
            ("_mixins.scss", "@import \"x\";"),
        ] {
            std::fs::write(dir.path().join(name), content).unwrap();
        }
        let found: Vec<String> = packages(&dir.path().join("app.css")).into_iter().collect();
        assert_eq!(found, ["@fontsource/inter", "tailwindcss"]);
        let found: Vec<String> = packages(&dir.path().join("x.scss")).into_iter().collect();
        assert_eq!(found, ["bootstrap"]);
        assert!(is_stylesheet(Path::new("a.less")) && !is_stylesheet(Path::new("a.js")));
        assert!(local(Path::new(""), "..").is_none());
    }
}
