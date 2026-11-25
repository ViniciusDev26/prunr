//! What one CommonJS file says about the module graph: what it requires, which names it uses
//! from each required module, and what it exports.
//!
//! A name is a path: `["a"]` is the export `a`, `["X", "m"]` is the static method `m` of the
//! exported class `X`. A module that assigns `module.exports` to something other than an object
//! literal has a single default export, `["default"]`.

use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, AssignmentExpression, AssignmentTarget, BindingPattern, CallExpression, Class,
    ClassElement, Expression, MethodDefinitionKind, ObjectPropertyKind, StaticMemberExpression,
};
use oxc_parser::Parser;
use oxc_semantic::{NodeId, Semantic, SemanticBuilder, SymbolId};
use oxc_span::{GetSpan, SourceType, Span};

/// A `require()` with a static specifier and how the file uses what it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Require {
    pub specifier: String,
    pub uses: Vec<Use>,
    /// Bindings the require declares that the file never references.
    pub unused: Vec<String>,
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
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::cjs()).parse();
    if let Some(error) = parsed.diagnostics.first() {
        return Err(error.to_string());
    }
    let program = allocator.alloc(parsed.program);
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let mut analyzer = Analyzer {
        semantic: &semantic,
        source,
        facts: Facts::default(),
        exports: BTreeMap::new(),
        reads: BTreeSet::new(),
    };
    for node in semantic.nodes().iter() {
        match node.kind() {
            AstKind::CallExpression(call) if analyzer.is_global(&call.callee, "require") => {
                analyzer.require(node.id(), call);
            }
            AstKind::AssignmentExpression(assignment) => analyzer.assignment(assignment),
            AstKind::StaticMemberExpression(member) => analyzer.read(node.id(), member),
            _ => {}
        }
    }
    Ok(analyzer.finish())
}

type Path = Vec<String>;

struct Analyzer<'s, 'a> {
    semantic: &'s Semantic<'a>,
    source: &'s str,
    facts: Facts,
    /// Every export, and whether the file uses it itself.
    exports: BTreeMap<Path, bool>,
    /// Exports read back through `exports.x` / `module.exports.x`.
    reads: BTreeSet<Path>,
}

impl<'a> Analyzer<'_, 'a> {
    fn finish(mut self) -> Facts {
        for path in &self.reads {
            if let Some(used) = self.exports.get_mut(path) {
                *used = true;
            }
        }
        self.facts.exports = self
            .exports
            .into_iter()
            .map(|(path, used_internally)| Export {
                path,
                used_internally,
            })
            .collect();
        self.facts
    }

    /// `expression` is the identifier `name`, unshadowed.
    fn is_global(&self, expression: &Expression, name: &str) -> bool {
        matches!(expression, Expression::Identifier(ident)
            if ident.name == name && self.semantic.is_reference_to_global_variable(ident))
    }

    fn is_module_exports(&self, member: &StaticMemberExpression) -> bool {
        member.property.name == "exports" && self.is_global(&member.object, "module")
    }

    /// `exports` or `module.exports`.
    fn is_exports_object(&self, expression: &Expression) -> bool {
        self.is_global(expression, "exports")
            || matches!(expression, Expression::StaticMemberExpression(member)
                if self.is_module_exports(member))
    }

    /// The path of a named export, which lives under the default export once there is one.
    fn named(&self, name: &str) -> Path {
        if self.facts.default_export {
            vec!["default".into(), name.into()]
        } else {
            vec![name.into()]
        }
    }

    // ---- require ----

    fn require(&mut self, node: NodeId, call: &CallExpression) {
        let specifier = match call.arguments.as_slice() {
            [Argument::StringLiteral(literal)] => Some(literal.value.display().to_string()),
            [Argument::TemplateLiteral(template)] => {
                template.single_quasi().map(|q| q.display().to_string())
            }
            _ => None,
        };
        let Some(specifier) = specifier else {
            let text = &self.source[call.span.start as usize..call.span.end as usize];
            self.facts.dynamic_requires.push(text.to_string());
            return;
        };
        let mut uses = BTreeSet::new();
        let mut unused = Vec::new();
        match self.semantic.nodes().parent_kind(node) {
            AstKind::VariableDeclarator(declarator) => {
                self.pattern_uses(&declarator.id, &mut uses, &mut unused);
            }
            AstKind::StaticMemberExpression(member) => {
                uses.insert(Use {
                    path: vec![member.property.name.to_string()],
                    whole: true,
                });
            }
            AstKind::CallExpression(outer) if outer.callee.span() == call.span => {
                uses.insert(Use {
                    path: vec![],
                    whole: false,
                });
            }
            AstKind::ExpressionStatement(_) => {}
            _ => {
                uses.insert(Use {
                    path: vec![],
                    whole: true,
                });
            }
        }
        self.facts.requires.push(Require {
            unused,
            specifier,
            uses: uses.into_iter().collect(),
        });
    }

    /// What a `const <pattern> = require(...)` uses; bindings never referenced go to `unused`.
    fn pattern_uses(
        &self,
        pattern: &BindingPattern,
        uses: &mut BTreeSet<Use>,
        unused: &mut Vec<String>,
    ) {
        let whole_module = Use {
            path: vec![],
            whole: true,
        };
        match pattern {
            BindingPattern::BindingIdentifier(ident) => {
                if !self.binding_uses(ident.symbol_id(), &[], uses) {
                    unused.push(ident.name.to_string());
                }
            }
            BindingPattern::ObjectPattern(object) => {
                if object.rest.is_some() {
                    uses.insert(whole_module.clone());
                }
                for property in &object.properties {
                    let Some(name) = property.key.static_name() else {
                        uses.insert(whole_module.clone());
                        continue;
                    };
                    let path = vec![name.display().to_string()];
                    if let BindingPattern::BindingIdentifier(ident) = &property.value {
                        if self.binding_uses(ident.symbol_id(), &path, uses) {
                            uses.insert(Use { path, whole: false });
                        } else {
                            unused.push(ident.name.to_string());
                        }
                    } else {
                        uses.insert(Use { path, whole: true });
                    }
                }
            }
            _ => {
                uses.insert(whole_module);
            }
        }
    }

    /// How the file uses a binding holding `prefix` of a required module. False when the file
    /// never references it.
    fn binding_uses(&self, symbol: SymbolId, prefix: &[String], uses: &mut BTreeSet<Use>) -> bool {
        let nodes = self.semantic.nodes();
        let mut referenced = false;
        for reference in self.semantic.symbol_references(symbol) {
            referenced = true;
            let node = reference.node_id();
            let span = nodes.kind(node).span();
            let used = match nodes.parent_kind(node) {
                AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                    let mut path = prefix.to_vec();
                    path.push(member.property.name.to_string());
                    Use { path, whole: true }
                }
                AstKind::CallExpression(call) if call.callee.span() == span => Use {
                    path: prefix.to_vec(),
                    whole: false,
                },
                AstKind::NewExpression(new) if new.callee.span() == span => Use {
                    path: prefix.to_vec(),
                    whole: false,
                },
                _ => Use {
                    path: prefix.to_vec(),
                    whole: true,
                },
            };
            uses.insert(used);
        }
        referenced
    }

    // ---- exports ----

    fn assignment(&mut self, assignment: &AssignmentExpression<'a>) {
        let AssignmentTarget::StaticMemberExpression(target) = &assignment.left else {
            return;
        };
        if self.is_module_exports(target) {
            self.module_exports(&assignment.right);
        } else if self.is_exports_object(&target.object) {
            let path = self.named(&target.property.name);
            self.export_value(path, &assignment.right);
        }
    }

    /// `module.exports = value`.
    fn module_exports(&mut self, value: &Expression<'a>) {
        if let Expression::ObjectExpression(object) = value {
            for property in &object.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property
                    && let Some(name) = property.key.static_name()
                {
                    self.export_value(vec![name.display().to_string()], &property.value);
                }
            }
        } else {
            self.facts.default_export = true;
            self.export_value(vec!["default".into()], value);
        }
    }

    fn export_value(&mut self, path: Path, value: &Expression<'a>) {
        self.exports.entry(path.clone()).or_insert(false);
        match value {
            Expression::Identifier(ident) => {
                let reference = self.semantic.scoping().get_reference(ident.reference_id());
                if let Some(symbol) = reference.symbol_id() {
                    self.export_symbol(path, symbol, ident.span);
                }
            }
            Expression::ClassExpression(class) => {
                self.export_statics(&path, class, &BTreeSet::new())
            }
            _ => {}
        }
    }

    /// An export whose value is the local `symbol`, referenced at `site` by the export itself.
    fn export_symbol(&mut self, path: Path, symbol: SymbolId, site: Span) {
        let semantic = self.semantic;
        let nodes = semantic.nodes();
        let mut members = BTreeSet::new();
        let mut used = false;
        for reference in semantic.symbol_references(symbol) {
            let node = reference.node_id();
            let span = nodes.kind(node).span();
            if span == site {
                continue;
            }
            used = true;
            if let AstKind::StaticMemberExpression(member) = nodes.parent_kind(node)
                && member.object.span() == span
            {
                members.insert(member.property.name.to_string());
            }
        }
        if used {
            self.exports.insert(path.clone(), true);
        }
        if let AstKind::Class(class) = semantic.symbol_declaration(symbol).kind() {
            self.export_statics(&path, class, &members);
        }
    }

    /// The static methods of an exported class; `called` are the ones the file calls itself.
    fn export_statics(&mut self, path: &[String], class: &Class, called: &BTreeSet<String>) {
        for element in &class.body.body {
            if let ClassElement::MethodDefinition(method) = element
                && method.r#static
                && method.kind == MethodDefinitionKind::Method
                && let Some(name) = method.key.static_name()
            {
                let mut member = path.to_vec();
                let name = name.display().to_string();
                let used = called.contains(&name);
                member.push(name);
                self.exports.insert(member, used);
            }
        }
    }

    /// `exports.x` / `module.exports.x` read back by the file.
    fn read(&mut self, node: NodeId, member: &StaticMemberExpression) {
        let written = matches!(self.semantic.nodes().parent_kind(node),
            AstKind::AssignmentExpression(assignment) if assignment.left.span() == member.span);
        if !written && self.is_exports_object(&member.object) {
            self.reads.insert(self.named(&member.property.name));
        }
    }
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
        assert!(facts.requires.iter().all(|r| r.unused.is_empty()));
        let specifiers: Vec<_> = facts
            .requires
            .iter()
            .map(|r| r.specifier.as_str())
            .collect();
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
        assert_eq!(
            facts(r#"function f(require) { require("./a"); }"#).requires,
            []
        );
    }

    #[test]
    fn a_dynamic_require_is_reported_by_its_source() {
        let facts = facts("const name = 'x'; require(name); require(`./${name}`); require();");
        assert_eq!(facts.requires, []);
        assert_eq!(
            facts.dynamic_requires,
            ["require(name)", "require(`./${name}`)", "require()"]
        );
    }

    // ---- how a require is used ----

    #[test]
    fn destructuring_uses_each_property_it_references() {
        assert_eq!(
            uses_of(r#"const { a, b: c } = require("./x"); use(a, c);"#),
            [
                uses(&["a"], false),
                uses(&["a"], true),
                uses(&["b"], false),
                uses(&["b"], true)
            ]
        );
    }

    fn unused_of(source: &str) -> Vec<String> {
        let facts = facts(source);
        assert_eq!(facts.requires.len(), 1, "{facts:?}");
        facts.requires[0].unused.clone()
    }

    #[test]
    fn a_destructured_binding_never_referenced_is_unused_and_uses_nothing() {
        let source = r#"const { a, b: c, d } = require("./x"); d();"#;
        assert_eq!(uses_of(source), [uses(&["d"], false)]);
        assert_eq!(unused_of(source), ["a", "c"]);
    }

    #[test]
    fn a_namespace_binding_never_referenced_is_unused() {
        assert_eq!(unused_of(r#"const x = require("./x");"#), ["x"]);
    }

    #[test]
    fn a_require_for_its_side_effect_declares_nothing_unused() {
        assert!(unused_of(r#"require("./a");"#).is_empty());
        assert!(unused_of(r#"const x = require("./a"); x.run();"#).is_empty());
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
            uses_of(r#"const { a, ...rest } = require("./x"); a();"#),
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
        assert_eq!(
            facts.exports,
            [export(&["a"], false), export(&["b"], false)]
        );
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
            [
                export(&["default"], false),
                export(&["default", "helper"], false)
            ]
        );
    }

    #[test]
    fn the_static_methods_of_an_exported_class_are_exports() {
        let facts = facts(
            "class X { static m() {} static get g() { return 1; } n() {} static p = 1; }
             module.exports = { X };",
        );
        assert_eq!(
            facts.exports,
            [export(&["X"], false), export(&["X", "m"], false)]
        );
    }

    #[test]
    fn the_static_methods_of_a_default_class_are_exports() {
        for source in [
            "class X { static m() {} } module.exports = X;",
            "module.exports = class { static m() {} };",
        ] {
            assert_eq!(
                facts(source).exports,
                [
                    export(&["default"], false),
                    export(&["default", "m"], false)
                ],
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
        let facts =
            facts("class X { static m() {} static n() { return X.m(); } } module.exports = X;");
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
    fn reading_a_name_that_was_never_exported_is_not_an_export() {
        let facts = facts("exports.b = 1; exports.a();");
        assert_eq!(facts.exports, [export(&["b"], false)]);
    }

    #[test]
    fn exporting_a_global_is_an_export_the_file_does_not_use() {
        let facts = facts("module.exports = { log: console, env: process.env };");
        assert_eq!(
            facts.exports,
            [export(&["env"], false), export(&["log"], false)]
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
