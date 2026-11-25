//! A configuration file read without running it: the object it exports, as JSON.

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrayExpressionElement, AssignmentTarget, BindingPattern, Expression, ObjectPropertyKind,
    Statement,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde_json::Value;

/// The object a configuration file exports, as JSON: literals are kept, anything only known at
/// runtime is `null`. `None` when the file does not export an object.
pub fn static_config(source: &str, path: &std::path::Path) -> Option<Value> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or(SourceType::cjs());
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    let mut constants = BTreeMap::new();
    let mut exported = None;
    for statement in &parsed.program.body {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                        (&declarator.id, &declarator.init)
                    {
                        constants.insert(id.name.as_str(), init);
                    }
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                exported = export.declaration.as_expression();
            }
            Statement::ExpressionStatement(statement) => {
                if let Expression::AssignmentExpression(assignment) = &statement.expression
                    && let AssignmentTarget::StaticMemberExpression(target) = &assignment.left
                    && target.property.name == "exports"
                    && matches!(&target.object, Expression::Identifier(id) if id.name == "module")
                {
                    exported = Some(&assignment.right);
                }
            }
            _ => {}
        }
    }
    match value(exported?, &constants) {
        object @ Value::Object(_) => Some(object),
        _ => None,
    }
}

/// The JSON value of a literal expression; `null` for anything else.
fn value(expression: &Expression, constants: &BTreeMap<&str, &Expression>) -> Value {
    match expression.get_inner_expression() {
        Expression::StringLiteral(literal) => Value::String(literal.value.display().to_string()),
        Expression::TemplateLiteral(template) => {
            template.single_quasi().map_or(Value::Null, |quasi| {
                Value::String(quasi.display().to_string())
            })
        }
        Expression::NumericLiteral(number) => serde_json::json!(number.value),
        Expression::BooleanLiteral(boolean) => Value::Bool(boolean.value),
        Expression::ArrayExpression(array) => Value::Array(
            array
                .elements
                .iter()
                .map(|element| match element {
                    ArrayExpressionElement::SpreadElement(_)
                    | ArrayExpressionElement::Elision(_) => Value::Null,
                    element => value(element.to_expression(), constants),
                })
                .collect(),
        ),
        Expression::ObjectExpression(object) => Value::Object(
            object
                .properties
                .iter()
                .filter_map(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        let key = property.key.static_name()?.display().to_string();
                        Some((key, value(&property.value, constants)))
                    }
                    ObjectPropertyKind::SpreadProperty(_) => None,
                })
                .collect(),
        ),
        Expression::CallExpression(call) => match call.arguments.first() {
            Some(argument) if call.arguments.len() == 1 => match argument.as_expression() {
                Some(argument) => value(argument, constants),
                None => Value::Null,
            },
            _ => Value::Null,
        },
        Expression::Identifier(id) => constants
            .get(id.name.as_str())
            .map_or(Value::Null, |init| value(init, constants)),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config(path: &str, source: &str) -> Option<Value> {
        static_config(source, std::path::Path::new(path))
    }

    #[test]
    fn module_exports_of_an_object_is_the_config() {
        assert_eq!(
            config(
                "jest.config.js",
                r#"module.exports = {
                    a: "x", b: ['y', `z`], c: { d: true, e: false }, f: 1.5, g: null,
                    h: someVariable, [computed]: 1, ...spread, i: [...rest, "j"]
                };"#
            ),
            Some(json!({
                "a": "x", "b": ["y", "z"], "c": { "d": true, "e": false }, "f": 1.5,
                "g": null, "h": null, "i": [null, "j"]
            }))
        );
    }

    #[test]
    fn export_default_through_define_config_and_type_assertions_is_the_config() {
        assert_eq!(
            config(
                "vitest.config.ts",
                r#"import { defineConfig } from "vitest/config";
                   export default defineConfig({ test: { include: ["a"] } }) satisfies object;"#
            ),
            Some(json!({ "test": { "include": ["a"] } }))
        );
    }

    #[test]
    fn a_config_held_in_a_constant_is_followed() {
        assert_eq!(
            config(
                "jest.config.ts",
                r#"const config = { preset: "ts-jest" } as const;
                   let other = 1, unused;
                   export default config;"#
            ),
            Some(json!({ "preset": "ts-jest" }))
        );
    }

    #[test]
    fn a_config_built_at_runtime_is_unknown() {
        for source in [
            "module.exports = async () => ({});",
            "export default defineConfig(() => ({}));",
            "export default missing;",
            "exports.preset = 'ts-jest';",
            "module.exports = { a: 1 };;; 1 + 1;",
            "export default merge(...configs);",
            "export default merge(a, b);",
            "export default merge();",
        ] {
            let expected = if source.contains("a: 1") {
                Some(json!({ "a": 1.0 }))
            } else {
                None
            };
            assert_eq!(config("jest.config.js", source), expected, "{source}");
        }
    }

    #[test]
    fn a_config_that_does_not_parse_is_unknown() {
        assert_eq!(config("jest.config.js", "module.exports = {"), None);
    }
}
