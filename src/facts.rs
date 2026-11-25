//! What one file says about the module graph: what it requires or imports, which names it uses
//! from each of those modules, and what it exports. CommonJS, ESM and TypeScript alike.
//!
//! A name is a path: `["a"]` is the export `a`, `["X", "m"]` is the static method `m` of the
//! exported class `X`. A module that assigns `module.exports` to something other than an object
//! literal has a single default export, `["default"]`; an ESM `export default` is the export
//! `["default"]` next to the named ones.

use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, BinaryOperator, BindingIdentifier, BindingPattern,
    CallExpression, Class, ClassElement, Declaration, ExportAllDeclaration, ExportDeclaration,
    ExportDefaultDeclaration, ExportDefaultDeclarationKind, ExportFromDeclaration,
    ExportNamedDeclaration, Expression, IdentifierReference, ImportDeclaration,
    ImportDeclarationSpecifier, ImportExpression, MethodDefinitionKind, ObjectPropertyKind,
    StaticMemberExpression, TSImportEqualsDeclaration, TSModuleReference, TemplateLiteral,
};
use oxc_parser::Parser;
use oxc_semantic::{NodeId, Semantic, SemanticBuilder, SymbolId};
use oxc_span::{GetSpan, SourceType, Span};

/// A `require()` or `import` with a static specifier and how the file uses what it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Require {
    pub specifier: String,
    pub uses: Vec<Use>,
    /// Bindings the require declares that the file never references.
    pub unused: Vec<String>,
    /// An ESM `import`: its uses are of the module namespace, where a CommonJS module is the
    /// `default`.
    pub esm: bool,
    /// `import type`: it is gone once TypeScript compiles.
    pub type_only: bool,
}

/// `export ... from`: names of another module this one exports as its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reexport {
    pub specifier: String,
    pub kind: ReexportKind,
    pub type_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReexportKind {
    /// `export { imported as exported } from`.
    Named { imported: String, exported: String },
    /// `export * from`: every named export.
    All,
    /// `export * as name from`.
    Namespace(String),
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

/// A `require()` whose specifier is built at runtime from a relative prefix, as a glob relative
/// to the file: `` require(`./handlers/${name}`) `` → `./handlers/*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    /// The source text of the `require()`.
    pub source: String,
    pub glob: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Facts {
    pub requires: Vec<Require>,
    pub exports: Vec<Export>,
    /// `module.exports` is assigned something other than an object literal.
    pub default_export: bool,
    /// The file has ESM `import` or `export` declarations.
    pub esm: bool,
    pub reexports: Vec<Reexport>,
    /// `require()`s whose specifier is a pattern; each uses every file it matches whole.
    pub patterns: Vec<Pattern>,
    /// Source text of each `require()` whose specifier is neither a static string nor a pattern.
    pub dynamic_requires: Vec<String>,
}

/// Analyzes one source, parsed as its path's extension says. Fails with the parser's first
/// diagnostic.
pub fn analyze(source: &str, path: &std::path::Path) -> Result<Facts, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or(SourceType::cjs());
    let parsed = Parser::new(&allocator, source, source_type).parse();
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
            AstKind::ImportDeclaration(import) => analyzer.import(import),
            AstKind::ImportExpression(import) => analyzer.dynamic_import(import),
            AstKind::TSImportEqualsDeclaration(import) => analyzer.import_equals(import),
            AstKind::ExportNamedDeclaration(export) => analyzer.export_named(export),
            AstKind::ExportFromDeclaration(export) => analyzer.export_from(export),
            AstKind::ExportDeclaration(export) => analyzer.export_declaration(export),
            AstKind::ExportDefaultDeclaration(export) => analyzer.export_default(export),
            AstKind::ExportAllDeclaration(export) => analyzer.export_all(export),
            AstKind::TSExportAssignment(export) => analyzer.module_exports(&export.expression),
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
        let argument = match call.arguments.as_slice() {
            [argument] => argument.as_expression(),
            _ => None,
        };
        let Some(specifier) = argument.and_then(specifier) else {
            self.runtime_specifier(call.span, argument);
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
            esm: false,
            type_only: false,
        });
    }

    /// A `require()` or `import()` at `span` whose specifier is only known at runtime: a pattern
    /// when it has a relative prefix, dynamic otherwise.
    fn runtime_specifier(&mut self, span: Span, argument: Option<&Expression>) {
        let source = self.source[span.start as usize..span.end as usize].to_string();
        match argument.and_then(pattern) {
            Some(glob) => self.facts.patterns.push(Pattern { source, glob }),
            None => self.facts.dynamic_requires.push(source),
        }
    }

    /// A binding that holds `path` of a module: what the file uses through it, or unused.
    fn named_binding(
        &self,
        ident: &BindingIdentifier,
        path: Path,
        uses: &mut BTreeSet<Use>,
        unused: &mut Vec<String>,
    ) {
        if self.binding_uses(ident.symbol_id(), &path, uses) {
            uses.insert(Use { path, whole: false });
        } else {
            unused.push(ident.name.to_string());
        }
    }

    // ---- import ----

    fn import(&mut self, import: &ImportDeclaration) {
        self.facts.esm = true;
        let mut uses = BTreeSet::new();
        let mut unused = Vec::new();
        let specifiers = import
            .specifiers
            .as_deref()
            .map_or(&[][..], |s| s.as_slice());
        for specifier in specifiers {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(named) => {
                    let path = vec![named.imported.name().to_string()];
                    self.named_binding(&named.local, path, &mut uses, &mut unused);
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                    let path = vec!["default".to_string()];
                    self.named_binding(&default.local, path, &mut uses, &mut unused);
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                    if !self.binding_uses(namespace.local.symbol_id(), &[], &mut uses) {
                        unused.push(namespace.local.name.to_string());
                    }
                }
            }
        }
        let every_specifier_is_a_type = !specifiers.is_empty()
            && specifiers.iter().all(|specifier| {
                matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(named)
                    if named.import_kind.is_type())
            });
        self.facts.requires.push(Require {
            specifier: import.source.value.display().to_string(),
            uses: uses.into_iter().collect(),
            unused,
            esm: true,
            type_only: import.import_kind.is_type() || every_specifier_is_a_type,
        });
    }

    /// `import()`: the whole module is used.
    fn dynamic_import(&mut self, import: &ImportExpression) {
        let Some(specifier) = specifier(&import.source) else {
            self.runtime_specifier(import.span, Some(&import.source));
            return;
        };
        self.facts.requires.push(Require {
            specifier,
            uses: vec![Use {
                path: vec![],
                whole: true,
            }],
            unused: Vec::new(),
            esm: true,
            type_only: false,
        });
    }

    /// `import x = require("./x")`, TypeScript's CommonJS import.
    fn import_equals(&mut self, import: &TSImportEqualsDeclaration) {
        let TSModuleReference::ExternalModuleReference(reference) = &import.module_reference else {
            return;
        };
        let mut uses = BTreeSet::new();
        let mut unused = Vec::new();
        if !self.binding_uses(import.id.symbol_id(), &[], &mut uses) {
            unused.push(import.id.name.to_string());
        }
        self.facts.requires.push(Require {
            specifier: reference.expression.value.display().to_string(),
            uses: uses.into_iter().collect(),
            unused,
            esm: false,
            type_only: import.import_kind.is_type(),
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
                        self.named_binding(ident, path, uses, unused);
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
            Expression::Identifier(ident) => self.export_reference(path, ident),
            Expression::ClassExpression(class) => {
                self.export_statics(&path, class, &BTreeSet::new())
            }
            _ => {}
        }
    }

    /// An export whose value is the local `ident` refers to, when it refers to one.
    fn export_reference(&mut self, path: Path, ident: &IdentifierReference) {
        let reference = self.semantic.scoping().get_reference(ident.reference_id());
        if let Some(symbol) = reference.symbol_id() {
            self.export_symbol(path, symbol, ident.span);
        }
    }

    /// An export of the local `ident` declares.
    fn export_binding(&mut self, path: Path, ident: &BindingIdentifier) {
        self.exports.entry(path.clone()).or_insert(false);
        self.export_symbol(path, ident.symbol_id(), ident.span);
    }

    /// An export whose value is the local `symbol`, referenced at `site` by the export itself.
    /// Other exports of the same symbol are not uses either.
    fn export_symbol(&mut self, path: Path, symbol: SymbolId, site: Span) {
        let semantic = self.semantic;
        let nodes = semantic.nodes();
        let mut members = BTreeSet::new();
        let mut used = false;
        for reference in semantic.symbol_references(symbol) {
            let node = reference.node_id();
            let span = nodes.kind(node).span();
            let exported = matches!(
                nodes.parent_kind(node),
                AstKind::ExportSpecifier(_) | AstKind::ExportDefaultDeclaration(_)
            );
            if span == site || exported {
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

    // ---- ESM exports ----

    fn export_named(&mut self, export: &ExportNamedDeclaration<'a>) {
        self.facts.esm = true;
        for specifier in &export.specifiers {
            let path = vec![specifier.exported.name().to_string()];
            self.exports.entry(path.clone()).or_insert(false);
            let local = specifier.local.name();
            if let Some(symbol) = self
                .semantic
                .scoping()
                .get_root_binding(local.as_str().into())
            {
                self.export_symbol(path, symbol, specifier.local.span());
            }
        }
    }

    /// `export { a as b } from "./x"`.
    fn export_from(&mut self, export: &ExportFromDeclaration) {
        self.facts.esm = true;
        for specifier in &export.specifiers {
            self.facts.reexports.push(Reexport {
                specifier: export.source.value.display().to_string(),
                kind: ReexportKind::Named {
                    imported: specifier.local.name().to_string(),
                    exported: specifier.exported.name().to_string(),
                },
                type_only: export.export_kind.is_type() || specifier.export_kind.is_type(),
            });
        }
    }

    /// `export const a = 1`, `export function f() {}`, `export type T = ...` and the like.
    fn export_declaration(&mut self, export: &ExportDeclaration<'a>) {
        self.facts.esm = true;
        let idents = match &export.declaration {
            Declaration::VariableDeclaration(variables) => variables
                .declarations
                .iter()
                .flat_map(|declarator| declarator.id.get_binding_identifiers())
                .collect(),
            Declaration::FunctionDeclaration(function) => function.id.iter().collect(),
            Declaration::ClassDeclaration(class) => class.id.iter().collect(),
            Declaration::TSTypeAliasDeclaration(alias) => vec![&alias.id],
            Declaration::TSInterfaceDeclaration(interface) => vec![&interface.id],
            Declaration::TSEnumDeclaration(enumeration) => vec![&enumeration.id],
            Declaration::TSNamespaceDeclaration(namespace) => vec![&namespace.id],
            _ => Vec::new(),
        };
        for ident in idents {
            self.export_binding(vec![ident.name.to_string()], ident);
        }
    }

    fn export_default(&mut self, export: &ExportDefaultDeclaration<'a>) {
        self.facts.esm = true;
        let path = vec!["default".to_string()];
        let id = match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => function.id.as_ref(),
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                if class.id.is_none() {
                    self.export_statics(&path, class, &BTreeSet::new());
                }
                class.id.as_ref()
            }
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => None,
            expression => {
                let expression = expression
                    .as_expression()
                    .expect("the rest are expressions");
                self.export_value(path, expression);
                return;
            }
        };
        match id {
            Some(id) => self.export_binding(path, id),
            None => {
                self.exports.entry(path).or_insert(false);
            }
        }
    }

    fn export_all(&mut self, export: &ExportAllDeclaration) {
        self.facts.esm = true;
        let kind = match &export.exported {
            Some(name) => ReexportKind::Namespace(name.name().to_string()),
            None => ReexportKind::All,
        };
        self.facts.reexports.push(Reexport {
            specifier: export.source.value.display().to_string(),
            kind,
            type_only: export.export_kind.is_type(),
        });
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

/// A specifier known before running: a string, or a template without expressions.
fn specifier(expression: &Expression) -> Option<String> {
    match expression {
        Expression::StringLiteral(literal) => Some(literal.value.display().to_string()),
        Expression::TemplateLiteral(template) => {
            template.single_quasi().map(|q| q.display().to_string())
        }
        _ => None,
    }
}

/// The glob a specifier built at runtime can match, when it starts with a relative path.
fn pattern(expression: &Expression) -> Option<String> {
    let mut parts = Vec::new();
    pattern_parts(expression, &mut parts);
    match parts.first() {
        Some(Some(prefix)) if prefix.starts_with("./") || prefix.starts_with("../") => {}
        _ => return None,
    }
    let mut glob = String::new();
    for part in parts {
        match part {
            Some(text) => glob.extend(text.chars().map(|c| match c {
                '*' | '?' | '[' | ']' | '{' | '}' => format!("[{c}]"),
                c => c.to_string(),
            })),
            None if glob.ends_with('*') && !glob.ends_with("[*]") => {}
            None => glob.push('*'),
        }
    }
    Some(glob)
}

/// The literal text (`Some`) and runtime values (`None`) a specifier is concatenated from.
fn pattern_parts(expression: &Expression, parts: &mut Vec<Option<String>>) {
    match expression {
        Expression::StringLiteral(literal) => parts.push(Some(literal.value.display().to_string())),
        Expression::TemplateLiteral(template) => template_parts(template, parts),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            pattern_parts(&binary.left, parts);
            pattern_parts(&binary.right, parts);
        }
        _ => parts.push(None),
    }
}

fn template_parts(template: &TemplateLiteral, parts: &mut Vec<Option<String>>) {
    for (index, quasi) in template.quasis.iter().enumerate() {
        let text = quasi
            .value
            .cooked
            .map(|c| c.display().to_string())
            .unwrap_or_default();
        if !text.is_empty() {
            parts.push(Some(text));
        }
        if index < template.expressions.len() {
            parts.push(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(source: &str) -> Facts {
        facts_at("index.js", source)
    }

    fn facts_at(path: &str, source: &str) -> Facts {
        analyze(source, std::path::Path::new(path)).unwrap()
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
        let facts = facts("const name = 'x'; require(name); require(`${name}/x`); require();");
        assert_eq!(facts.requires, []);
        assert_eq!(
            facts.dynamic_requires,
            ["require(name)", "require(`${name}/x`)", "require()"]
        );
    }

    fn patterns_of(source: &str) -> Vec<(String, String)> {
        let facts = facts(source);
        assert_eq!(facts.requires, []);
        assert_eq!(facts.dynamic_requires, Vec::<String>::new());
        facts
            .patterns
            .into_iter()
            .map(|p| (p.source, p.glob))
            .collect()
    }

    fn pattern(source: &str, glob: &str) -> (String, String) {
        (source.to_string(), glob.to_string())
    }

    #[test]
    fn a_template_require_with_a_relative_prefix_is_a_pattern() {
        assert_eq!(
            patterns_of("require(`./handlers/${a}/${b}.js`); require(`../up/${a}${b}`);"),
            [
                pattern("require(`./handlers/${a}/${b}.js`)", "./handlers/*/*.js"),
                pattern("require(`../up/${a}${b}`)", "../up/*"),
            ]
        );
    }

    #[test]
    fn a_concatenated_require_with_a_relative_prefix_is_a_pattern() {
        assert_eq!(
            patterns_of(r#"require("./jobs/" + name + ".job.js"); require('./a/' + `b` + x);"#),
            [
                pattern(
                    r#"require("./jobs/" + name + ".job.js")"#,
                    "./jobs/*.job.js"
                ),
                pattern("require('./a/' + `b` + x)", "./a/b*"),
            ]
        );
    }

    #[test]
    fn glob_characters_in_a_pattern_are_literal() {
        assert_eq!(
            patterns_of("require(`./[x]{y}?*/${a}`);"),
            [pattern(
                "require(`./[x]{y}?*/${a}`)",
                "./[[]x[]][{]y[}][?][*]/*"
            )]
        );
    }

    #[test]
    fn a_pattern_without_a_relative_prefix_is_dynamic() {
        let facts = facts(r#"require(`/abs/${a}`); require(a + "./x"); require("x" - a);"#);
        assert_eq!(facts.patterns, []);
        assert_eq!(
            facts.dynamic_requires,
            [
                "require(`/abs/${a}`)",
                r#"require(a + "./x")"#,
                r#"require("x" - a)"#
            ]
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

    // ---- ESM imports ----

    fn esm_require(facts: &Facts) -> &Require {
        assert_eq!(facts.requires.len(), 1, "{facts:?}");
        assert!(facts.esm);
        assert!(facts.requires[0].esm);
        &facts.requires[0]
    }

    #[test]
    fn a_default_import_uses_default() {
        let facts = facts(r#"import a from "./a"; a(); a.x;"#);
        assert_eq!(
            esm_require(&facts).uses,
            [uses(&["default"], false), uses(&["default", "x"], true)]
        );
    }

    #[test]
    fn a_named_import_uses_its_name() {
        let facts = facts(r#"import { b, c as d } from "./b"; b(); d.y;"#);
        assert_eq!(
            esm_require(&facts).uses,
            [
                uses(&["b"], false),
                uses(&["c"], false),
                uses(&["c", "y"], true)
            ]
        );
        assert_eq!(
            facts_at("a.mjs", r#"import { "s t" as e } from "./b"; e();"#).requires[0].uses,
            [uses(&["s t"], false)]
        );
    }

    #[test]
    fn a_namespace_import_uses_the_members_it_reads() {
        let facts = facts(r#"import * as ns from "./c"; ns.e(); ns.f;"#);
        assert_eq!(
            esm_require(&facts).uses,
            [uses(&["e"], true), uses(&["f"], true)]
        );
    }

    #[test]
    fn an_import_for_its_side_effect_uses_nothing() {
        let facts = facts(r#"import "./x";"#);
        assert_eq!(esm_require(&facts).uses, []);
        assert!(facts.requires[0].unused.is_empty());
    }

    #[test]
    fn an_import_binding_never_referenced_is_unused() {
        let facts = facts(r#"import a, { b, c } from "./a"; import * as ns from "./n"; c();"#);
        assert_eq!(facts.requires[0].unused, ["a", "b"]);
        assert_eq!(facts.requires[0].uses, [uses(&["c"], false)]);
        assert_eq!(facts.requires[1].unused, ["ns"]);
    }

    #[test]
    fn a_require_is_not_esm() {
        let facts = facts(r#"require("./a");"#);
        assert!(!facts.esm);
        assert!(!facts.requires[0].esm);
    }

    #[test]
    fn an_import_of_types_only_is_type_only() {
        let source = r#"
            import type { A } from "./a";
            import { type B } from "./b";
            import { type C, D } from "./c";
            import E from "./e";
            let x: A | B | C | D | E;
        "#;
        let facts = facts_at("index.ts", source);
        let type_only: Vec<bool> = facts.requires.iter().map(|r| r.type_only).collect();
        assert_eq!(type_only, [true, true, false, false]);
        assert_eq!(
            facts.requires[0].uses,
            [uses(&["A"], false), uses(&["A"], true)]
        );
    }

    #[test]
    fn a_dynamic_import_uses_the_whole_module() {
        let facts =
            facts("async function f(x) { await import('./a'); import(`./p/${x}`); import(x); }");
        assert!(!facts.esm, "import() is valid in CommonJS too");
        assert!(facts.requires[0].esm);
        assert_eq!(facts.requires[0].uses, [uses(&[], true)]);
        assert_eq!(facts.requires[0].specifier, "./a");
        assert_eq!(facts.patterns.len(), 1);
        assert_eq!(facts.patterns[0].glob, "./p/*");
        assert_eq!(facts.dynamic_requires, ["import(x)"]);
    }

    #[test]
    fn import_equals_require_is_a_require() {
        let facts = facts_at(
            "index.ts",
            r#"import x = require("./a"); x.m(); import y = N.y;"#,
        );
        assert_eq!(facts.requires.len(), 1);
        assert!(!facts.requires[0].esm);
        assert_eq!(facts.requires[0].uses, [uses(&["m"], true)]);
        assert_eq!(
            facts_at("index.ts", r#"import z = require("./z");"#).requires[0].unused,
            ["z"]
        );
    }

    #[test]
    fn a_jsx_element_uses_its_component() {
        let facts = facts_at(
            "index.tsx",
            r#"import C from "./C"; import * as ui from "./ui"; export const x = <C><ui.B /></C>;"#,
        );
        assert_eq!(
            facts.requires[0].uses,
            [uses(&["default"], false), uses(&["default"], true)]
        );
        assert_eq!(facts.requires[1].uses, [uses(&[], true)]);
    }

    // ---- ESM exports ----

    #[test]
    fn exported_declarations_are_exports() {
        let facts = facts_at(
            "index.ts",
            "export const a = 1, { b } = o; export function f() {}
             export class C { static m() {} } export type T = 1; export interface I {}
             export enum E { X } export namespace N {} export declare module 'm' {}",
        );
        assert!(facts.esm);
        assert!(!facts.default_export);
        assert_eq!(
            facts.exports,
            [
                export(&["C"], false),
                export(&["C", "m"], false),
                export(&["E"], false),
                export(&["I"], false),
                export(&["N"], false),
                export(&["T"], false),
                export(&["a"], false),
                export(&["b"], false),
                export(&["f"], false),
            ]
        );
    }

    #[test]
    fn export_specifiers_and_default_are_exports() {
        let facts = facts("const a = 1; export { a, a as b, c as 'd' }; export default a;");
        assert_eq!(
            facts.exports,
            [
                export(&["a"], false),
                export(&["b"], false),
                export(&["d"], false),
                export(&["default"], false),
            ]
        );
    }

    #[test]
    fn a_default_declaration_is_the_default_export() {
        let exports = |path: &str, source: &str| facts_at(path, source).exports;
        assert_eq!(
            exports("a.js", "export default function f() {} f();"),
            [export(&["default"], true)]
        );
        assert_eq!(
            exports("a.js", "export default class { static m() {} }"),
            [
                export(&["default"], false),
                export(&["default", "m"], false)
            ]
        );
        for (path, source) in [
            ("a.js", "export default function () {}"),
            ("a.ts", "export default interface I {}"),
            ("a.js", "export default 42;"),
            ("a.js", "export default class C {}"),
        ] {
            assert_eq!(
                exports(path, source),
                [export(&["default"], false)],
                "{source}"
            );
        }
    }

    #[test]
    fn an_exported_declaration_used_in_its_file_is_used_internally() {
        let facts = facts_at(
            "index.ts",
            "export function f() {} export type T = 1; f(); let x: T;",
        );
        assert_eq!(facts.exports, [export(&["T"], true), export(&["f"], true)]);
    }

    #[test]
    fn export_assignment_is_a_default_export() {
        let facts = facts_at("index.ts", "function f() {} export = f;");
        assert!(facts.default_export);
        assert_eq!(facts.exports, [export(&["default"], false)]);
    }

    #[test]
    fn reexports_name_their_source() {
        let facts = facts_at(
            "index.ts",
            "export { a, b as c, default } from './x'; export * from './y';
             export * as ns from './z'; export type { T } from './t'; export type * from './u';",
        );
        assert!(facts.esm);
        assert_eq!(facts.exports, []);
        let named = |imported: &str, exported: &str| ReexportKind::Named {
            imported: imported.into(),
            exported: exported.into(),
        };
        let reexport = |specifier: &str, kind, type_only| Reexport {
            specifier: specifier.into(),
            kind,
            type_only,
        };
        assert_eq!(
            facts.reexports,
            [
                reexport("./x", named("a", "a"), false),
                reexport("./x", named("b", "c"), false),
                reexport("./x", named("default", "default"), false),
                reexport("./y", ReexportKind::All, false),
                reexport("./z", ReexportKind::Namespace("ns".into()), false),
                reexport("./t", named("T", "T"), true),
                reexport("./u", ReexportKind::All, true),
            ]
        );
    }

    #[test]
    fn typescript_syntax_parses_in_typescript_files() {
        let facts = facts_at(
            "index.mts",
            "import { a } from './a'; const x: number = a<string>(); export { x };",
        );
        assert_eq!(facts.exports, [export(&["x"], false)]);
        assert!(analyze("const x: number = 1;", std::path::Path::new("index.js")).is_err());
    }

    #[test]
    fn a_syntax_error_fails_the_analysis() {
        let error = analyze("const = ;", std::path::Path::new("index.js")).unwrap_err();
        assert!(!error.is_empty());
    }
}
