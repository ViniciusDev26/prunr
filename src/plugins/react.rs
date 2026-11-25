//! React: JSX compiles to imports of `react/jsx-runtime` the code never writes, so `react` is
//! used in production even without an `import React`.

use super::{Context, Contribution, Plugin};

pub struct React;

impl Plugin for React {
    fn name(&self) -> &'static str {
        "react"
    }

    fn contribute(&self, _context: &Context, contribution: &mut Contribution) {
        contribution.production_packages.insert("react".into());
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn react_is_used_by_the_jsx_it_compiles() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["react"]), &json!({}));
        assert_eq!(found.production_packages, declared(&["react"]));
        assert!(found.production.is_empty() && found.packages.is_empty());
    }
}
