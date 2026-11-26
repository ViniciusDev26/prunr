//! chartjs-node-canvas: the Chart.js plugins it loads are named in its options, and it requires
//! them itself (`new ChartJSNodeCanvas({ plugins: { modern: ["chartjs-plugin-annotation"] } })`).

use super::{Context, Contribution, Plugin};

pub struct ChartJsNodeCanvas;

/// The keys of its `plugins` option that name packages it requires.
const KEYS: &[&str] = &[
    "modern",
    "requireLegacy",
    "globalVariableLegacy",
    "requireChartJSLegacy",
];

impl Plugin for ChartJsNodeCanvas {
    fn name(&self) -> &'static str {
        "chartjs-node-canvas"
    }

    fn contribute(&self, _context: &Context, contribution: &mut Contribution) {
        for key in KEYS {
            contribution
                .loaded_by_name
                .push((self.name().to_string(), key.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::*;
    use serde_json::json;

    #[test]
    fn the_plugins_it_requires_are_named_under_its_keys() {
        let dir = project(&[]);
        let found = setup(dir.path(), &declared(&["chartjs-node-canvas"]), &json!({}));
        assert_eq!(found.loaded_by_name.len(), 4);
        assert_eq!(
            found.loaded_by_name[0],
            ("chartjs-node-canvas".to_string(), "modern".to_string())
        );
    }
}
