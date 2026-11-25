//! What one CommonJS file says about the module graph: what it requires, which names it uses
//! from each required module, and what it exports.
//!
//! A name is a path: `["a"]` is the export `a`, `["X", "m"]` is the static method `m` of the
//! exported class `X`. A module that assigns `module.exports` to something other than an object
//! literal has a single default export, `["default"]`.

use std::collections::{BTreeMap, BTreeSet};

/// A `require()` with a static specifier and how the file uses what it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Require {
    pub specifier: String,
    pub uses: Vec<Use>,
}

/// One use of a required module. With `whole`, everything under `path` is used too.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Use {
    pub path: Vec<String>,
    pub whole: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub path: Vec<String>,
    /// The file itself uses it, so it is alive even when no other file does.
    pub used_internally: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Facts {
    pub requires: Vec<Require>,
    pub exports: Vec<Export>,
    /// `module.exports` is assigned something other than an object literal.
    pub default_export: bool,
    /// Source text of each `require()` whose specifier is not a static string.
    pub dynamic_requires: Vec<String>,
}

/// Analyzes one CommonJS source. Fails with the parser's first diagnostic.
pub fn analyze(source: &str) -> Result<Facts, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(source: &str) -> Facts {
        analyze(source).unwrap()
    }

    fn uses(path: &[&str], whole: bool) -> Use {
        Use {
            path: path.iter().map(|s| s.to_string()).collect(),
            whole,
        }
    }

    fn uses_of(source: &str) -> Vec<Use> {
        let facts = facts(source);
        assert_eq!(facts.requires.len(), 1, "{facts:?}");
        facts.requires[0].uses.clone()
    }

    fn export(path: &[&str], used_internally: bool) -> Export {
        Export {
            path: path.iter().map(|s| s.to_string()).collect(),
            used_internally,
        }
    }

    // ---- require ----

    #[test]
    fn records_each_static_require_in_order() {
        let facts = facts(r#"require("./a"); require("pkg"); require(`./b`);"#);
        let specifiers: Vec<_> = facts.requires.iter().map(|r| r.specifier.as_str()).collect();
        assert_eq!(specifiers, ["./a", "pkg", "./b"]);
    }

    #[test]
    fn a_require_for_its_side_effect_uses_nothing() {
        assert_eq!(uses_of(r#"require("./a");"#), []);
    }

    #[test]
    fn a_require_inside_a_function_counts() {
        let facts = facts(r#"function f() { return require("./a").x; }"#);
        assert_eq!(facts.requires[0].specifier, "./a");
    }

    #[test]
    fn a_shadowed_require_is_not_a_require() {
        assert_eq!(facts(r#"function f(require) { require("./a"); }"#).requires, []);
    }

    #[test]
    fn a_dynamic_require_is_reported_by_its_source() {
        let facts = facts("const name = 'x'; require(name); require(`./${name}`); require();");
        assert_eq!(facts.requires, []);
        assert_eq!(facts.dynamic_requires, ["require(name)", "require(`./${name}`)", "require()"]);
    }

    // ---- how a require is used ----

    #[test]
    fn destructuring_uses_each_property() {
        assert_eq!(
            uses_of(r#"const { a, b: c } = require("./x");"#),
            [uses(&["a"], false), uses(&["b"], false)]
        );
    }

    #[test]
    fn a_member_of_a_destructured_binding_is_used() {
        assert_eq!(
            uses_of(r#"const { X } = require("./x"); X.m(); new X();"#),
            [uses(&["X"], false), uses(&["X", "m"], true)]
        );
    }

    #[test]
    fn a_destructured_binding_that_escapes_is_used_whole() {
        assert_eq!(
            uses_of(r#"const { X } = require("./x"); use(X);"#),
            [uses(&["X"], false), uses(&["X"], true)]
        );
    }

    #[test]
    fn a_rest_element_uses_the_whole_module() {
        assert_eq!(
            uses_of(r#"const { a, ...rest } = require("./x");"#),
            [uses(&[], true), uses(&["a"], false)]
        );
    }

    #[test]
    fn a_computed_or_nested_property_is_used_conservatively() {
        assert_eq!(
            uses_of(r#"const { [key]: a, b: { c }, d = 1 } = require("./x");"#),
            [uses(&[], true), uses(&["b"], true), uses(&["d"], true)]
        );
    }

    #[test]
    fn an_array_pattern_uses_the_whole_module() {
        assert_eq!(uses_of(r#"const [a] = require("./x");"#), [uses(&[], true)]);
    }

    #[test]
    fn a_member_of_a_namespace_binding_is_used() {
        assert_eq!(
            uses_of(r#"const x = require("./x"); x.a(); x.b;"#),
            [uses(&["a"], true), uses(&["b"], true)]
        );
    }

    #[test]
    fn calling_a_namespace_binding_uses_the_module_itself() {
        assert_eq!(
            uses_of(r#"const f = require("./f"); f(); new f();"#),
            [uses(&[], false)]
        );
    }

    #[test]
    fn a_namespace_binding_that_escapes_is_used_whole() {
        assert_eq!(
            uses_of(r#"const x = require("./x"); use(x);"#),
            [uses(&[], true)]
        );
    }

    #[test]
    fn an_unreferenced_namespace_binding_uses_nothing() {
        assert_eq!(uses_of(r#"const x = require("./x");"#), []);
    }

    #[test]
    fn a_member_of_the_require_call_is_used() {
        assert_eq!(uses_of(r#"require("./x").a();"#), [uses(&["a"], true)]);
    }

    #[test]
    fn calling_the_require_call_uses_the_module_itself() {
        assert_eq!(uses_of(r#"require("./f")();"#), [uses(&[], false)]);
    }

    #[test]
    fn a_require_in_any_other_position_is_used_whole() {
        assert_eq!(uses_of(r#"use(require("./x"));"#), [uses(&[], true)]);
        assert_eq!(uses_of(r#"let x; x = require("./x");"#), [uses(&[], true)]);
    }

    // ---- exports ----

    #[test]
    fn an_object_literal_exports_its_keys() {
        let facts = facts(
            r#"const a = 1; const impl = 2; const k = "k";
               module.exports = { a, b: impl, c() {}, "d": 4, [k]: 5, ...other };"#,
        );
        assert!(!facts.default_export);
        assert_eq!(
            facts.exports,
            [
                export(&["a"], false),
                export(&["b"], false),
                export(&["c"], false),
                export(&["d"], false),
            ]
        );
    }

    #[test]
    fn property_assignments_export_their_names() {
        let facts = facts("exports.a = 1; module.exports.b = 2; exports.a = 3;");
        assert_eq!(facts.exports, [export(&["a"], false), export(&["b"], false)]);
    }

    #[test]
    fn any_other_value_is_a_default_export() {
        for source in [
            "module.exports = function () {};",
            "module.exports = 42;",
            "function f() {} module.exports = f;",
        ] {
            let facts = facts(source);
            assert!(facts.default_export, "{source}");
            assert_eq!(facts.exports, [export(&["default"], false)], "{source}");
        }
    }

    #[test]
    fn a_property_added_to_a_default_export_lives_under_it() {
        let facts = facts("module.exports = function () {}; module.exports.helper = 1;");
        assert_eq!(
            facts.exports,
            [export(&["default"], false), export(&["default", "helper"], false)]
        );
    }

    #[test]
    fn the_static_methods_of_an_exported_class_are_exports() {
        let facts = facts(
            "class X { static m() {} static get g() { return 1; } n() {} static p = 1; }
             module.exports = { X };",
        );
        assert_eq!(facts.exports, [export(&["X"], false), export(&["X", "m"], false)]);
    }

    #[test]
    fn the_static_methods_of_a_default_class_are_exports() {
        for source in [
            "class X { static m() {} } module.exports = X;",
            "module.exports = class { static m() {} };",
        ] {
            assert_eq!(
                facts(source).exports,
                [export(&["default"], false), export(&["default", "m"], false)],
                "{source}"
            );
        }
    }

    #[test]
    fn an_export_referenced_in_its_own_file_is_used_internally() {
        let facts = facts("function a() {} function b() { a(); } module.exports = { a, b };");
        assert_eq!(facts.exports, [export(&["a"], true), export(&["b"], false)]);
    }

    #[test]
    fn reading_an_export_back_is_internal_use() {
        let facts = facts("exports.a = () => 1; exports.b = () => exports.a() + module.exports.b;");
        assert_eq!(facts.exports, [export(&["a"], true), export(&["b"], true)]);
    }

    #[test]
    fn a_static_method_called_in_its_own_file_is_used_internally() {
        let facts = facts(
            "class X { static m() {} static n() { return X.m(); } } module.exports = X;",
        );
        assert_eq!(
            facts.exports,
            [
                export(&["default"], true),
                export(&["default", "m"], true),
                export(&["default", "n"], false),
            ]
        );
    }

    #[test]
    fn shadowed_module_and_exports_are_not_exports() {
        let facts = facts("function f(exports, module) { exports.a = 1; module.exports = {}; }");
        assert_eq!(facts.exports, []);
        assert!(!facts.default_export);
    }

    #[test]
    fn a_syntax_error_fails_the_analysis() {
        let error = analyze("const = ;").unwrap_err();
        assert!(!error.is_empty());
    }
}
