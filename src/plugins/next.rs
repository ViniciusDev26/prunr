//! Next.js: its routes are entries by convention, and it loads React without an `import`.
//!
//! An entry's exports are its interface, so what Next.js reads from a route (`default`,
//! `getServerSideProps`, `metadata`, `generateStaticParams`, `GET`...) is never reported.

use super::{Context, Contribution, Plugin};

pub struct Next;

/// The files of the app router Next.js loads by name.
const APP_FILES: &str = "page,layout,route,loading,error,global-error,not-found,template,\
default,opengraph-image,twitter-image,icon,apple-icon,sitemap,robots,manifest";

/// The files at the root of the project (or of `src/`) Next.js loads by name.
const ROOT_FILES: &[&str] = &["middleware", "instrumentation", "instrumentation-client"];

impl Plugin for Next {
    fn name(&self) -> &'static str {
        "next"
    }

    fn contribute(&self, _context: &Context, contribution: &mut Contribution) {
        for base in ["", "src/"] {
            contribution.production.push(format!("{base}pages/**"));
            contribution
                .production
                .push(format!("{base}app/**/{{{APP_FILES}}}.*"));
            for file in ROOT_FILES {
                contribution.production.push(format!("{base}{file}.*"));
            }
        }
        contribution.production.push("next.config.*".into());
        for package in ["react", "react-dom"] {
            contribution.production_packages.insert(package.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn routes_pages_and_root_files_are_production_entries() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["next"]), &json!({}));
        for glob in [
            "pages/**",
            "src/pages/**",
            "middleware.*",
            "src/instrumentation.*",
            "next.config.*",
        ] {
            assert!(found.production.contains(&glob.to_string()), "{glob}");
        }
        assert!(
            found
                .production
                .iter()
                .any(|glob| glob.starts_with("app/**/{page,"))
        );
        assert_eq!(found.production_packages, declared(&["react", "react-dom"]));
        assert!(found.tests.is_empty() && found.files.is_empty());
    }
}
