//! What one file says about the module graph: what it requires or imports, which names it uses
//! from each of those modules, and what it exports. CommonJS, ESM and TypeScript alike.
//!
//! A name is a path: `["a"]` is the export `a`, `["X", "m"]` is the static method `m` of the
//! exported class `X`. A module that assigns `module.exports` to something other than an object
//! literal has a single default export, `["default"]`; an ESM `export default` is the export
//! `["default"]` next to the named ones.

use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrayExpressionElement, AssignmentExpression, AssignmentTarget, BinaryOperator,
    BindingIdentifier, BindingPattern, CallExpression, Class, ClassElement, Declaration,
    ExportAllDeclaration, ExportDeclaration, ExportDefaultDeclaration,
    ExportDefaultDeclarationKind, ExportFromDeclaration, ExportNamedDeclaration, Expression,
    IdentifierReference, ImportDeclaration, ImportDeclarationSpecifier, ImportExpression,
    MethodDefinitionKind, ObjectPropertyKind, PropertyKey, StaticMemberExpression, TSAccessibility,
    TSEnumDeclaration, TSImportEqualsDeclaration, TSModuleReference, TemplateLiteral,
    UnaryOperator, VariableDeclarator,
};
use oxc_ast::{AstKind, Comment};
use oxc_parser::{ParseOptions, Parser};
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
    /// The exports that are members of an exported enum (`["E", "A"]`).
    pub enum_members: BTreeSet<Vec<String>>,
    /// The public methods of an exported class (`["C", "m"]`). Without types, any read of a
    /// property with the same name, on any object, uses one.
    pub class_members: BTreeSet<Vec<String>>,
    /// The names of the properties the file reads (`a.b`, `const { b } = a`, `a["b"]`).
    pub properties: BTreeSet<String>,
    /// The relative paths written as strings (`"./setup.ts"`), with their lines: in a config a
    /// tool reads, they name the files it loads.
    pub path_strings: Vec<(String, u32)>,
    /// Groups of ESM exports that are the same local binding (`export { a, a as b }`), by
    /// name; `default` is not one.
    pub duplicate_exports: Vec<Vec<String>>,
    /// The file has ESM `import` or `export` declarations.
    pub esm: bool,
    pub reexports: Vec<Reexport>,
    /// `require()`s whose specifier is a pattern; each uses every file it matches whole.
    pub patterns: Vec<Pattern>,
    /// Source text of each `require()` whose specifier is neither a static string nor a pattern.
    pub dynamic_requires: Vec<String>,
    /// The line (from 1) where each export is made.
    pub export_lines: BTreeMap<Vec<String>, u32>,
    /// The first line that requires, imports or reexports each specifier; for a specifier only
    /// known at runtime, the key is the source text of the call.
    pub require_lines: BTreeMap<String, u32>,
    /// Exports a comment says not to report.
    pub ignores: BTreeMap<Vec<String>, Ignore>,
    /// The line of a `prunr-ignore-file` comment: the file and its exports are not reported.
    pub ignore_file: Option<u32>,
    /// Lines of `prunr-ignore-next-line` comments whose next line makes no export.
    pub stray_ignores: Vec<u32>,
    /// Where each ignore comment is, by its line.
    pub ignore_comments: BTreeMap<u32, Span>,
    /// How to stop exporting each export.
    pub export_removals: BTreeMap<Vec<String>, Removal>,
    /// How to drop each import binding, by specifier and local name.
    pub import_removals: BTreeMap<(String, String), Removal>,
    /// The declaration of the local an export refers to (`module.exports = { a }`), with the
    /// comment right above it, when nothing else in the file reads it and it has no side
    /// effects: removing every export of it leaves it unused.
    pub export_declarations: BTreeMap<Vec<String>, Span>,
}

/// How `prunr fix` takes an export or an import binding out of the code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    /// Delete these bytes: the `export` before a declaration, shared by the names it declares.
    Prefix(Span),
    /// Delete this statement or class member, as whole lines.
    Statement(Span),
    /// Drop item `index` of the comma-separated list at `items`. When every item goes, delete
    /// `all` as whole lines, or just the items when there is no `all`.
    Item {
        items: Vec<Span>,
        index: usize,
        all: Option<Span>,
    },
    /// Drop specifier `index` of the ESM import at `statement`, which is rebuilt from `parts`;
    /// when every specifier goes, the statement is deleted.
    Import {
        statement: Span,
        index: usize,
        parts: ImportParts,
    },
}

/// An ESM import, in pieces of source text to rebuild it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportParts {
    /// `import` or `import type`.
    pub keyword: String,
    pub specifiers: Vec<(Specifier, String)>,
    /// The quoted specifier, as written.
    pub source: String,
    pub semicolon: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Specifier {
    Default,
    Namespace,
    Named,
}

/// Why an export is not reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ignore {
    /// A `prunr-ignore-next-line` comment at this line. It must ignore something: on an export
    /// that is used, it is reported.
    NextLine(u32),
    /// A JSDoc comment with `@public`: API on purpose, documented as such.
    Public,
}

/// Analyzes one source, parsed as its path's extension says. Fails with the parser's first
/// diagnostic.
pub fn analyze(source: &str, path: &std::path::Path) -> Result<Facts, String> {
    analyze_with(source, path, false)
}

/// Analyzes the scripts of a markup file (a `.vue` component): a binding they never reference
/// still uses what it was given, since the markup around them can use it.
pub fn analyze_markup(source: &str, path: &std::path::Path) -> Result<Facts, String> {
    analyze_with(source, path, true)
}

fn analyze_with(source: &str, path: &std::path::Path, markup: bool) -> Result<Facts, String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or(SourceType::cjs());
    // JSX is common in `.js` files (Create React App, webpack examples), and a CommonJS module
    // can `return` at its top level.
    let source_type = match source_type.is_typescript() {
        true => source_type,
        false => source_type.with_jsx(true),
    };
    let options = ParseOptions {
        allow_return_outside_function: true,
        ..ParseOptions::default()
    };
    let parsed = Parser::new(&allocator, source, source_type)
        .with_options(options)
        .parse();
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
        markup,
        facts: Facts::default(),
        exports: BTreeMap::new(),
        reads: BTreeSet::new(),
        sites: BTreeMap::new(),
        site: Span::default(),
        require_sites: BTreeMap::new(),
        removal: None,
        exports_escape: false,
        exported_symbols: BTreeSet::new(),
        path_strings: Vec::new(),
        exports_of: BTreeMap::new(),
        passed_symbols: BTreeSet::new(),
        comments: program
            .comments
            .iter()
            .map(|comment| comment.span)
            .collect(),
    };
    for node in semantic.nodes().iter() {
        // A `namespace` (or `declare module`) exports to itself, not to the module.
        let in_namespace = || {
            semantic
                .nodes()
                .ancestor_kinds(node.id())
                .any(|kind| matches!(kind, AstKind::TSModuleBlock(_)))
        };
        match node.kind() {
            AstKind::CallExpression(call) if analyzer.is_global(&call.callee, "require") => {
                analyzer.require(node.id(), call);
            }
            AstKind::CallExpression(call) => analyzer.call_arguments(call),
            AstKind::VariableDeclarator(declarator) => analyzer.declarator(declarator),
            AstKind::AssignmentExpression(assignment) => analyzer.assignment(node.id(), assignment),
            AstKind::StaticMemberExpression(member) => {
                analyzer.property(&member.property.name);
                analyzer.read(node.id(), member)
            }
            AstKind::StringLiteral(literal) => {
                let value = literal.value.display().to_string();
                if value.starts_with("./") || value.starts_with("../") {
                    analyzer.path_strings.push((value, literal.span.start));
                }
            }
            AstKind::ComputedMemberExpression(member) => {
                if let Expression::StringLiteral(name) = &member.expression {
                    analyzer.property(&name.value.display().to_string());
                }
            }
            AstKind::ObjectPattern(pattern) => {
                for property in &pattern.properties {
                    if let Some(name) = property.key.static_name() {
                        analyzer.property(&name.display().to_string());
                    }
                }
            }
            AstKind::ImportDeclaration(import) => analyzer.import(import),
            AstKind::ImportExpression(import) => analyzer.dynamic_import(import),
            AstKind::TSImportEqualsDeclaration(import) => analyzer.import_equals(import),
            AstKind::ExportNamedDeclaration(export) if !in_namespace() => {
                analyzer.export_named(export)
            }
            AstKind::ExportFromDeclaration(export) => analyzer.export_from(export),
            AstKind::ExportDeclaration(export) if !in_namespace() => {
                analyzer.export_declaration(export)
            }
            AstKind::ExportDefaultDeclaration(export) => analyzer.export_default(export),
            AstKind::ExportAllDeclaration(export) => analyzer.export_all(export),
            AstKind::TSExportAssignment(export) => {
                analyzer.site = export.span;
                analyzer.removal = Some(Removal::Statement(export.span));
                analyzer.module_exports(&export.expression);
            }
            _ => {}
        }
    }
    analyzer.jsdoc_imports(&program.comments);
    Ok(analyzer.finish(&program.comments))
}

type Path = Vec<String>;

struct Analyzer<'s, 'a> {
    semantic: &'s Semantic<'a>,
    source: &'s str,
    /// The scripts of a markup file: their bindings are used even without a reference.
    markup: bool,
    facts: Facts,
    /// Every export, and whether the file uses it itself.
    exports: BTreeMap<Path, bool>,
    /// Exports read back through `exports.x` / `module.exports.x`.
    reads: BTreeSet<Path>,
    /// Where each export is made: the statement, property or method a comment before it
    /// refers to.
    sites: BTreeMap<Path, Span>,
    /// The site of the exports being declared.
    site: Span,
    /// Where each specifier is first required, imported or reexported.
    require_sites: BTreeMap<String, u32>,
    /// How to remove the exports being declared.
    removal: Option<Removal>,
    /// Every comment, in source order.
    comments: Vec<Span>,
    /// The file passes its own `exports` to a function, which can read any of them.
    exports_escape: bool,
    /// The bindings that are the exported value (`module.exports = api`,
    /// `var God = module.exports = {...}`).
    exported_symbols: BTreeSet<SymbolId>,
    /// The relative paths written as strings, with their offsets.
    path_strings: Vec<(String, u32)>,
    /// The top-level exports of each local binding: more than one is a duplicate.
    exports_of: BTreeMap<SymbolId, BTreeSet<String>>,
    /// The bindings the file passes to a function.
    passed_symbols: BTreeSet<SymbolId>,
}

/// Methods the language or the platform calls by name: never reported.
const CONVENTIONAL_METHODS: &[&str] = &[
    "toString",
    "toJSON",
    "valueOf",
    "toLocaleString",
    "then",
    "inspect",
    "handleEvent",
];

impl<'a> Analyzer<'_, 'a> {
    fn finish(mut self, comments: &[Comment]) -> Facts {
        for path in &self.reads {
            if let Some(used) = self.exports.get_mut(path) {
                *used = true;
            }
        }
        if self.exports_escape || !self.exported_symbols.is_disjoint(&self.passed_symbols) {
            self.exports.values_mut().for_each(|used| *used = true);
        }
        let newlines: Vec<usize> = self.source.match_indices('\n').map(|(at, _)| at).collect();
        let line = |offset: u32| newlines.partition_point(|&at| at < offset as usize) as u32 + 1;
        self.facts.export_lines = self
            .sites
            .iter()
            .map(|(path, site)| (path.clone(), line(site.start)))
            .collect();
        self.facts.path_strings = self
            .path_strings
            .iter()
            .map(|(path, offset)| (path.clone(), line(*offset)))
            .collect();
        self.facts.require_lines = self
            .require_sites
            .iter()
            .map(|(specifier, offset)| (specifier.clone(), line(*offset)))
            .collect();
        for comment in comments {
            let text = &self.source[comment.span.start as usize..comment.span.end as usize];
            let at = line(comment.span.start);
            if text.contains("prunr-ignore-file") {
                self.facts.ignore_file.get_or_insert(at);
                self.facts.ignore_comments.insert(at, comment.span);
                continue;
            }
            let ignore = if text.contains("prunr-ignore-next-line") {
                self.facts.ignore_comments.insert(at, comment.span);
                Ignore::NextLine(at)
            } else if comment.is_jsdoc() && text.contains("@public") {
                Ignore::Public
            } else {
                continue;
            };
            let next = line(comment.span.end) + 1;
            let targets: Vec<&Path> = self
                .facts
                .export_lines
                .iter()
                .filter(|(_, line)| **line == next)
                .map(|(path, _)| path)
                .collect();
            if targets.is_empty() && ignore != Ignore::Public {
                self.facts.stray_ignores.push(at);
            }
            for path in targets {
                self.facts.ignores.insert(path.clone(), ignore.clone());
            }
        }
        // Only ESM: a CommonJS object is often a table (`{ "<a>": parse, "<b>": parse }`).
        self.facts.duplicate_exports = self
            .exports_of
            .into_values()
            .filter(|names| names.len() > 1 && self.facts.esm)
            .map(|names| names.into_iter().collect())
            .collect();
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
        self.required(&specifier, call.span);
        let mut uses = BTreeSet::new();
        let mut unused = Vec::new();
        match self.semantic.nodes().parent_kind(node) {
            AstKind::VariableDeclarator(declarator) => {
                let nodes = self.semantic.nodes();
                let statement = match nodes.parent_kind(nodes.parent_id(node)) {
                    AstKind::VariableDeclaration(declaration)
                        if declaration.declarations.len() == 1 =>
                    {
                        Some(declaration.span)
                    }
                    _ => None,
                };
                let mut removals = Vec::new();
                self.pattern_uses(
                    &declarator.id,
                    statement,
                    &mut uses,
                    &mut unused,
                    &mut removals,
                );
                for (name, removal) in removals {
                    self.facts
                        .import_removals
                        .insert((specifier.clone(), name), removal);
                }
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
        self.required(&source, span);
        match argument.and_then(pattern) {
            Some(glob) => self.facts.patterns.push(Pattern { source, glob }),
            None => self.facts.dynamic_requires.push(source),
        }
    }

    /// `import("./x").Name` in JSDoc comments: a require of `./x` that only types use, and so
    /// can form no cycle. Without a member, the whole module is used.
    fn jsdoc_imports(&mut self, comments: &[Comment]) {
        for comment in comments.iter().filter(|comment| comment.is_jsdoc()) {
            let start = comment.span.start as usize;
            let text = &self.source[start..comment.span.end as usize];
            let types = type_expressions(text);
            let in_type = |at: usize| types.iter().any(|(from, to)| *from < at && at < *to);
            for (at, _) in text.match_indices("import(").filter(|(at, _)| in_type(*at)) {
                let inner = text[at + "import(".len()..].trim_start();
                let Some(quote) = inner
                    .chars()
                    .next()
                    .filter(|c| matches!(c, '"' | '\'' | '`'))
                else {
                    continue;
                };
                let Some(length) = inner[1..].find(quote) else {
                    continue;
                };
                let specifier = &inner[1..1 + length];
                let Some(after) = inner[length + 2..].trim_start().strip_prefix(')') else {
                    continue;
                };
                let member: String = after
                    .strip_prefix('.')
                    .unwrap_or_default()
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                    .collect();
                let path = if member.is_empty() {
                    vec![]
                } else {
                    vec![member]
                };
                let offset = (start + at) as u32;
                self.required(specifier, Span::new(offset, offset));
                self.facts.requires.push(Require {
                    specifier: specifier.to_string(),
                    uses: vec![Use { path, whole: true }],
                    unused: Vec::new(),
                    esm: false,
                    type_only: true,
                });
            }
        }
    }

    /// Records where `specifier` is first required.
    fn required(&mut self, specifier: &str, site: Span) {
        self.require_sites
            .entry(specifier.to_string())
            .or_insert(site.start);
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
        let text = |span: Span| self.source[span.start as usize..span.end as usize].to_string();
        let source = import.source.value.display().to_string();
        if import.with_clause.is_none() && import.phase.is_none() {
            let parts = ImportParts {
                keyword: if import.import_kind.is_type() {
                    "import type".into()
                } else {
                    "import".into()
                },
                specifiers: specifiers
                    .iter()
                    .map(|specifier| {
                        let kind = match specifier {
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                                Specifier::Default
                            }
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                                Specifier::Namespace
                            }
                            ImportDeclarationSpecifier::ImportSpecifier(_) => Specifier::Named,
                        };
                        (kind, text(specifier.span()))
                    })
                    .collect(),
                source: text(import.source.span),
                semicolon: text(import.span).ends_with(';'),
            };
            for (index, specifier) in specifiers.iter().enumerate() {
                let removal = Removal::Import {
                    statement: import.span,
                    index,
                    parts: parts.clone(),
                };
                self.facts.import_removals.insert(
                    (source.clone(), specifier.local().name.to_string()),
                    removal,
                );
            }
        }
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
        self.required(&import.source.value.display().to_string(), import.span);
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
        self.required(&specifier, import.span);
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
        self.facts.import_removals.insert(
            (
                reference.expression.value.display().to_string(),
                import.id.name.to_string(),
            ),
            Removal::Statement(import.span),
        );
        self.required(
            &reference.expression.value.display().to_string(),
            import.span,
        );
        self.facts.requires.push(Require {
            specifier: reference.expression.value.display().to_string(),
            uses: uses.into_iter().collect(),
            unused,
            esm: false,
            type_only: import.import_kind.is_type(),
        });
    }

    /// What a `const <pattern> = require(...)` uses; bindings never referenced go to `unused`.
    /// `statement` is the declaration to delete with the last binding, when it declares nothing
    /// else; how to remove each binding goes to `removals`.
    fn pattern_uses(
        &self,
        pattern: &BindingPattern,
        statement: Option<Span>,
        uses: &mut BTreeSet<Use>,
        unused: &mut Vec<String>,
        removals: &mut Vec<(String, Removal)>,
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
                if let Some(statement) = statement {
                    removals.push((ident.name.to_string(), Removal::Statement(statement)));
                }
            }
            BindingPattern::ObjectPattern(object) => {
                if object.rest.is_some() {
                    uses.insert(whole_module.clone());
                }
                let items: Vec<Span> = object.properties.iter().map(|p| p.span).collect();
                for (index, property) in object.properties.iter().enumerate() {
                    if object.rest.is_none()
                        && let BindingPattern::BindingIdentifier(ident) = &property.value
                    {
                        let removal = Removal::Item {
                            items: items.clone(),
                            index,
                            all: statement,
                        };
                        removals.push((ident.name.to_string(), removal));
                    }
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
        if !referenced && self.markup {
            uses.insert(Use {
                path: prefix.to_vec(),
                whole: true,
            });
        }
        referenced || self.markup
    }

    // ---- exports ----

    /// `span` with the comments right above it: those with nothing but blanks and at most one
    /// line break between them and what follows. A comment after a blank line stays.
    fn with_leading_comments(&self, span: Span) -> Span {
        let mut start = span.start;
        for comment in self.comments.iter().rev() {
            if comment.end > start {
                continue;
            }
            let gap = &self.source[comment.end as usize..start as usize];
            if !gap.trim().is_empty() || gap.matches('\n').count() > 1 {
                break;
            }
            start = comment.start;
        }
        Span::new(start, span.end)
    }

    /// Records an export made at the current site.
    fn declare(&mut self, path: &Path) {
        self.exports.entry(path.clone()).or_insert(false);
        self.sites.entry(path.clone()).or_insert(self.site);
        if let Some(removal) = &self.removal {
            self.facts
                .export_removals
                .entry(path.clone())
                .or_insert_with(|| removal.clone());
        }
    }

    fn assignment(&mut self, node: NodeId, assignment: &AssignmentExpression<'a>) {
        let AssignmentTarget::StaticMemberExpression(target) = &assignment.left else {
            return;
        };
        self.site = assignment.span;
        self.removal = match self.semantic.nodes().parent_kind(node) {
            AstKind::ExpressionStatement(statement) => Some(Removal::Statement(statement.span)),
            _ => None,
        };
        if self.is_module_exports(target) {
            self.module_exports(&assignment.right);
        } else if self.is_exports_object(&target.object) {
            let path = self.named(&target.property.name);
            self.export_value(path, &assignment.right);
        }
    }

    /// `setup(exports)`, `extend(api)` where `api` is the exported value: the function can read
    /// any export.
    fn call_arguments(&mut self, call: &CallExpression<'a>) {
        for argument in call.arguments.iter().filter_map(|a| a.as_expression()) {
            if self.is_exports_object(argument) {
                self.exports_escape = true;
            } else if let Expression::Identifier(ident) = argument
                && let Some(symbol) = self.symbol_of(ident)
            {
                self.passed_symbols.insert(symbol);
            }
        }
    }

    /// `var God = module.exports = {...}`: `God` is the exported value.
    fn declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if let Some(Expression::AssignmentExpression(assignment)) = &declarator.init
            && let AssignmentTarget::StaticMemberExpression(target) = &assignment.left
            && self.is_module_exports(target)
            && let BindingPattern::BindingIdentifier(binding) = &declarator.id
        {
            self.exported_symbols.insert(binding.symbol_id());
        }
    }

    /// The binding an identifier refers to, if it is a local one.
    fn symbol_of(&self, ident: &IdentifierReference) -> Option<SymbolId> {
        self.semantic
            .scoping()
            .get_reference(ident.reference_id())
            .symbol_id()
    }

    /// `module.exports = value`.
    fn module_exports(&mut self, value: &Expression<'a>) {
        if let Expression::Identifier(ident) = value
            && let Some(symbol) = self.symbol_of(ident)
        {
            self.exported_symbols.insert(symbol);
        }
        if let Expression::ObjectExpression(object) = value {
            let items: Vec<Span> = object.properties.iter().map(|p| p.span()).collect();
            for (index, property) in object.properties.iter().enumerate() {
                if let ObjectPropertyKind::ObjectProperty(property) = property
                    && let Some(name) = property.key.static_name()
                {
                    self.site = property.span;
                    self.removal = Some(Removal::Item {
                        items: items.clone(),
                        index,
                        all: None,
                    });
                    self.export_value(vec![name.display().to_string()], &property.value);
                }
            }
        } else {
            self.facts.default_export = true;
            self.export_value(vec!["default".into()], value);
        }
    }

    fn export_value(&mut self, path: Path, value: &Expression<'a>) {
        self.declare(&path);
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
            self.export_local(path, symbol, ident.span);
        }
    }

    /// An export of a local declared elsewhere in the file (`exports.a = a`, `export { a }`):
    /// its declaration goes with the export when nothing else reads it and it has no side
    /// effects.
    fn export_local(&mut self, path: Path, symbol: SymbolId, site: Span) {
        self.export_symbol(path.clone(), symbol, site);
        if self.exports.get(&path) == Some(&true) {
            return;
        }
        let nodes = self.semantic.nodes();
        let declaration = self.semantic.symbol_declaration(symbol);
        let span = match declaration.kind() {
            AstKind::Function(function) if function.is_declaration() => Some(function.span),
            AstKind::Class(class) if class.is_declaration() => Some(class.span),
            AstKind::VariableDeclarator(declarator)
                if declarator.init.as_ref().is_none_or(is_pure) =>
            {
                match nodes.parent_kind(declaration.id()) {
                    AstKind::VariableDeclaration(variables)
                        if variables.declarations.len() == 1 =>
                    {
                        Some(variables.span)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let exported = matches!(
            nodes.parent_kind(nodes.parent_id(declaration.id())),
            AstKind::ExportDeclaration(_)
        ) || matches!(
            nodes.parent_kind(declaration.id()),
            AstKind::ExportDeclaration(_)
        );
        if let Some(span) = span
            && !exported
        {
            let span = self.with_leading_comments(span);
            self.facts.export_declarations.insert(path, span);
        }
    }

    /// An export of the local `ident` declares.
    fn export_binding(&mut self, path: Path, ident: &BindingIdentifier) {
        self.declare(&path);
        self.export_symbol(path, ident.symbol_id(), ident.span);
    }

    /// An export whose value is the local `symbol`, referenced at `site` by the export itself.
    /// Other exports of the same symbol are not uses either.
    fn export_symbol(&mut self, path: Path, symbol: SymbolId, site: Span) {
        // `export default a` next to `export { a }` is the usual way to offer both.
        if let [name] = path.as_slice()
            && name != "default"
        {
            self.exports_of
                .entry(symbol)
                .or_default()
                .insert(name.clone());
        }
        let semantic = self.semantic;
        let nodes = semantic.nodes();
        let mut members = BTreeSet::new();
        let mut used = false;
        // Read other than by a member (`Object.values(E)`, `E[key]`): every member is used.
        let mut whole = false;
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
            match nodes.parent_kind(node) {
                AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                    members.insert(member.property.name.to_string());
                }
                _ => whole = true,
            }
        }
        if used {
            self.exports.insert(path.clone(), true);
        }
        match semantic.symbol_declaration(symbol).kind() {
            AstKind::Class(class) => self.export_statics(&path, class, &members),
            AstKind::TSEnumDeclaration(enumeration) => {
                self.export_enum_members(&path, enumeration, &members, whole)
            }
            _ => {}
        }
    }

    /// The members of an exported enum; `read` are the ones the file reads itself, and
    /// `whole` says it reads them all.
    fn export_enum_members(
        &mut self,
        path: &[String],
        enumeration: &TSEnumDeclaration,
        read: &BTreeSet<String>,
        whole: bool,
    ) {
        let members = &enumeration.body.members;
        let items: Vec<Span> = members.iter().map(|member| member.span).collect();
        for (index, member) in members.iter().enumerate() {
            let name = member.id.static_name().display().to_string();
            let mut path = path.to_vec();
            let used = whole || read.contains(&name);
            path.push(name);
            self.sites.entry(path.clone()).or_insert(member.span);
            self.facts
                .export_removals
                .entry(path.clone())
                .or_insert(Removal::Item {
                    items: items.clone(),
                    index,
                    all: None,
                });
            self.facts.enum_members.insert(path.clone());
            self.exports.insert(path, used);
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
                self.sites.entry(member.clone()).or_insert(method.span);
                self.facts
                    .export_removals
                    .entry(member.clone())
                    .or_insert(Removal::Statement(method.span));
                self.exports.insert(member, used);
            }
        }
        self.export_methods(path, class);
    }

    /// The public methods of an exported class that only its own code defines: one that
    /// extends or implements something may be called through what it overrides, and a
    /// decorated one by the framework that reads the decorator.
    fn export_methods(&mut self, path: &[String], class: &Class) {
        if class.heritage.is_some() || !class.implements.is_empty() {
            return;
        }
        for element in &class.body.body {
            if let ClassElement::MethodDefinition(method) = element
                && !method.r#static
                && method.kind != MethodDefinitionKind::Constructor
                && method.decorators.is_empty()
                && method
                    .accessibility
                    .is_none_or(|a| a == TSAccessibility::Public)
                && let PropertyKey::StaticIdentifier(name) = &method.key
                && !CONVENTIONAL_METHODS.contains(&name.name.as_str())
            {
                let mut member = path.to_vec();
                member.push(name.name.to_string());
                self.sites.entry(member.clone()).or_insert(method.span);
                self.facts
                    .export_removals
                    .entry(member.clone())
                    .or_insert(Removal::Statement(method.span));
                self.facts.class_members.insert(member);
            }
        }
    }

    /// A property the file reads by name.
    fn property(&mut self, name: &str) {
        self.facts.properties.insert(name.to_string());
    }

    // ---- ESM exports ----

    fn export_named(&mut self, export: &ExportNamedDeclaration<'a>) {
        self.facts.esm = true;
        let items: Vec<Span> = export.specifiers.iter().map(|s| s.span).collect();
        for (index, specifier) in export.specifiers.iter().enumerate() {
            let path = vec![specifier.exported.name().to_string()];
            self.site = specifier.span;
            self.removal = Some(Removal::Item {
                items: items.clone(),
                index,
                all: Some(export.span),
            });
            self.declare(&path);
            let local = specifier.local.name();
            if let Some(symbol) = self
                .semantic
                .scoping()
                .get_root_binding(local.as_str().into())
            {
                self.export_local(path, symbol, specifier.local.span());
            }
        }
    }

    /// `export { a as b } from "./x"`.
    fn export_from(&mut self, export: &ExportFromDeclaration) {
        self.facts.esm = true;
        self.required(&export.source.value.display().to_string(), export.span);
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
        self.site = export.span;
        let pure = match &export.declaration {
            Declaration::VariableDeclaration(variables) => variables
                .declarations
                .iter()
                .all(|declarator| declarator.init.as_ref().is_none_or(is_pure)),
            _ => true,
        };
        self.removal = Some(if pure {
            Removal::Statement(self.with_leading_comments(export.span))
        } else {
            Removal::Prefix(Span::new(
                export.span.start,
                export.declaration.span().start,
            ))
        });
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
        self.site = export.span;
        let path = vec!["default".to_string()];
        let statement = Removal::Statement(self.with_leading_comments(export.span));
        self.removal = match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(_)
            | ExportDefaultDeclarationKind::ClassDeclaration(_)
            | ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => Some(statement),
            expression
                if expression
                    .as_expression()
                    .is_some_and(|expression| is_pure(expression)) =>
            {
                Some(statement)
            }
            _ => None,
        };
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
            None => self.declare(&path),
        }
    }

    fn export_all(&mut self, export: &ExportAllDeclaration) {
        self.facts.esm = true;
        self.required(&export.source.value.display().to_string(), export.span);
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

/// The type expressions of a JSDoc comment: the `{...}` right after a tag (`@type {X}`,
/// `@param {X} name`), as ranges of `text`. An `import("./x")` outside them is prose.
fn type_expressions(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for (at, _) in text.match_indices('@') {
        let tag = text[at + 1..].trim_start_matches(|c: char| c.is_ascii_alphabetic());
        let rest = tag.trim_start();
        if !rest.starts_with('{') || tag.len() == text.len() - at - 1 {
            continue;
        }
        let open = text.len() - rest.len();
        let mut depth = 0;
        for (offset, c) in text[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => continue,
            }
            if depth == 0 {
                ranges.push((open, open + offset));
                break;
            }
        }
    }
    ranges
}

/// Whether evaluating `expression` can have no side effect: a literal, a function, a class, or
/// an array or object of those (`Object.freeze` of one too).
fn is_pure(expression: &Expression) -> bool {
    match expression.get_inner_expression() {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::FunctionExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::ClassExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_pure(&unary.argument)
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
            element.as_expression().is_some_and(is_pure)
                || matches!(element, ArrayExpressionElement::Elision(_))
        }),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if !property.key.is_expression() && is_pure(&property.value))
        }),
        Expression::CallExpression(call) => {
            matches!(&call.callee, Expression::StaticMemberExpression(member)
                if member.property.name == "freeze"
                    && matches!(&member.object, Expression::Identifier(id) if id.name == "Object"))
                && call.arguments.len() == 1
                && call.arguments[0].as_expression().is_some_and(is_pure)
        }
        _ => false,
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
    fn passing_the_exported_value_to_a_function_uses_it_all() {
        for source in [
            "var God = module.exports = { next_id: 0, started_at: 1 }; require('./methods')(God);",
            "const api = { a: 1, b: 2 }; module.exports = api; extend(api);",
        ] {
            let found = facts(source);
            assert!(!found.exports.is_empty(), "{source}");
            assert!(found.exports.iter().all(|e| e.used_internally), "{source}");
        }
        let found =
            facts("const api = { a: 1 }; module.exports = api; const other = {}; f(other);");
        assert!(!found.exports.iter().all(|e| e.used_internally));
    }

    #[test]
    fn the_exports_of_a_namespace_are_its_members_not_the_module_s() {
        let found = facts_at(
            "socket.ts",
            "export class Socket {}\nexport namespace Socket {\n  export type Reason = 'a';\n  export const LIMIT = 1;\n}\ndeclare module 'x' {\n  export const y: number;\n}\n",
        );
        let paths: Vec<Vec<String>> = found.exports.iter().map(|e| e.path.clone()).collect();
        assert_eq!(paths, [vec!["Socket".to_string()]]);
    }

    #[test]
    fn only_an_import_in_a_jsdoc_type_is_an_import() {
        let found = facts(
            "/**\n * Example:\n *   import(\"./moduleA\");\n *   router({ slow: lazy(() => import('./slow.js')) })\n * @type {import('./types').Config}\n * @param {{ a: import(\"./b\").B }} x\n */\nexport const x = 1;\n",
        );
        let specifiers: Vec<&str> = found
            .requires
            .iter()
            .map(|r| r.specifier.as_str())
            .collect();
        assert_eq!(specifiers, ["./types", "./b"]);
    }

    #[test]
    fn passing_its_own_exports_to_a_function_uses_them_all() {
        for source in [
            "exports.a = 1; exports.b = 2; module.exports = require('./common')(exports);",
            "exports.a = 1; exports.b = 2; setup(module.exports);",
        ] {
            let found = facts(source);
            assert!(!found.exports.is_empty(), "{source}");
            assert!(found.exports.iter().all(|e| e.used_internally), "{source}");
        }
        let found = facts("exports.a = 1; exports.b = 2; log(exports.a);");
        assert!(!found.exports.iter().all(|e| e.used_internally));
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
    fn the_public_methods_of_an_exported_class_are_class_members() {
        let facts = facts_at(
            "index.ts",
            "export class A {
               constructor() {} m() {} get g() { return 1; } #p() {} private q() {}
               static s() {} f = 1; ['c']() {} @d() e() {} toString() { return ''; }
             }
             export class B extends Error { n() {} }
             export class C implements I { n() {} }
             export default class { o() {} }
             class D { r() {} } const x = 1; export { D, x };",
        );
        assert_eq!(
            facts.class_members,
            BTreeSet::from([
                path(&["A", "m"]),
                path(&["A", "g"]),
                path(&["D", "r"]),
                path(&["default", "o"]),
            ])
        );
        assert_eq!(facts.export_lines[&path(&["A", "m"])], 2);
        assert!(matches!(
            facts.export_removals[&path(&["A", "m"])],
            Removal::Statement(_)
        ));
    }

    #[test]
    fn the_properties_a_file_reads_are_known_by_name() {
        let facts = facts("a.b.c(); const { d, e: f } = g; h['i']; j[k]; ({ l: 1 });");
        assert_eq!(
            facts.properties,
            ["b", "c", "d", "e", "i"].map(String::from).into()
        );
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
                export(&["E", "X"], false),
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
    fn the_members_of_an_exported_enum_are_exports() {
        let facts = facts_at(
            "index.ts",
            "export enum E { A, B = 2, 'C' } export const a = E.A;
             enum F { X, Y } const all = Object.values(F); export { F, all };",
        );
        assert_eq!(
            facts.exports,
            [
                export(&["E"], true),
                export(&["E", "A"], true),
                export(&["E", "B"], false),
                export(&["E", "C"], false),
                export(&["F"], true),
                export(&["F", "X"], true),
                export(&["F", "Y"], true),
                export(&["a"], false),
                export(&["all"], false),
            ]
        );
        assert_eq!(
            facts.enum_members,
            BTreeSet::from([
                path(&["E", "A"]),
                path(&["E", "B"]),
                path(&["E", "C"]),
                path(&["F", "X"]),
                path(&["F", "Y"])
            ])
        );
        assert_eq!(facts.export_lines[&path(&["E", "B"])], 1);
        assert!(matches!(
            facts.export_removals[&path(&["E", "B"])],
            Removal::Item {
                index: 1,
                all: None,
                ..
            }
        ));
    }

    #[test]
    fn exports_of_the_same_binding_are_duplicates() {
        let esm = facts_at(
            "index.mjs",
            "function a() {} const c = 1; export { a, a as b, c }; export default a;",
        );
        assert_eq!(esm.duplicate_exports, [path(&["a", "b"])]);
        let commonjs = facts("function a() {} module.exports = { a, b: a, c: 1 };");
        assert!(commonjs.duplicate_exports.is_empty());
        let nested = facts("class X {} module.exports = X; module.exports.X = X;");
        assert!(nested.duplicate_exports.is_empty());
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

    // ---- ignore comments ----

    fn path(segments: &[&str]) -> Vec<String> {
        segments.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn each_export_knows_its_line() {
        let found = facts(
            "exports.a = 1;\nmodule.exports.b = 2;\nclass X {\n  static m() {}\n}\nexports.X = X;",
        );
        let lines: Vec<(Vec<String>, u32)> = found.export_lines.into_iter().collect();
        assert_eq!(
            lines,
            [
                (path(&["X"]), 6),
                (path(&["X", "m"]), 4),
                (path(&["a"]), 1),
                (path(&["b"]), 2)
            ]
        );
        let found = facts_at(
            "a.ts",
            "export const c = 1;\nexport {\n  c as d,\n};\nexport default c;\n",
        );
        assert_eq!(found.export_lines[&path(&["d"])], 3);
        assert_eq!(found.export_lines[&path(&["default"])], 5);
        let found = facts("module.exports = {\n  a,\n  b() {},\n};");
        assert_eq!(found.export_lines[&path(&["b"])], 3);
        let found = facts("function f() {}\nmodule.exports = f;");
        assert_eq!(found.export_lines[&path(&["default"])], 2);
        let found = facts_at("a.ts", "function f() {}\nexport = f;");
        assert_eq!(found.export_lines[&path(&["default"])], 2);
    }

    #[test]
    fn a_next_line_comment_ignores_the_exports_on_the_next_line() {
        let found = facts(
            "// prunr-ignore-next-line\nexport const a = 1, b = 2;\nexport const c = 3;\n\
             module.exports = {\n  /* prunr-ignore-next-line: hook */\n  d,\n};",
        );
        assert_eq!(
            found.ignores,
            BTreeMap::from([
                (path(&["a"]), Ignore::NextLine(1)),
                (path(&["b"]), Ignore::NextLine(1)),
                (path(&["d"]), Ignore::NextLine(5)),
            ])
        );
        assert!(found.stray_ignores.is_empty());
    }

    #[test]
    fn a_public_jsdoc_tag_ignores_the_export_it_documents() {
        let found = facts(
            "/**\n * API.\n * @public\n */\nexport function f() {}\n/* @public */\nexport const g = 1;",
        );
        assert_eq!(
            found.ignores,
            BTreeMap::from([(path(&["f"]), Ignore::Public)])
        );
    }

    #[test]
    fn a_next_line_comment_before_no_export_is_stray() {
        let found = facts(
            "// prunr-ignore-next-line\n\nexport const a = 1;\n// prunr-ignore-next-line\nconst b = 2;",
        );
        assert!(found.ignores.is_empty());
        assert_eq!(found.stray_ignores, [1, 4]);
    }

    #[test]
    fn a_file_comment_ignores_the_file() {
        assert_eq!(facts("exports.a = 1;").ignore_file, None);
        assert_eq!(
            facts("// header\n/* prunr-ignore-file: loaded by name */\nexports.a = 1;").ignore_file,
            Some(2)
        );
    }

    #[test]
    fn each_specifier_knows_the_first_line_that_requires_it() {
        let found = facts_at(
            "a.ts",
            "import a from './a';\nconst b = require('./b');\nrequire(x);\nrequire(`./p/${x}`);\n\
             import('./c');\nimport d = require('./d');\nexport { e } from './e';\n\
             export * from './f';\nrequire('./a');",
        );
        let lines: Vec<(&str, u32)> = found
            .require_lines
            .iter()
            .map(|(specifier, line)| (specifier.as_str(), *line))
            .collect();
        assert_eq!(
            lines,
            [
                ("./a", 1),
                ("./b", 2),
                ("./c", 5),
                ("./d", 6),
                ("./e", 7),
                ("./f", 8),
                ("require(`./p/${x}`)", 4),
                ("require(x)", 3),
            ]
        );
    }

    // ---- removals, for `prunr fix` ----

    /// The text of each span a removal names, and the index for an item.
    fn removal(source: &str, removal: &Removal) -> String {
        let text = |span: &Span| source[span.start as usize..span.end as usize].to_string();
        match removal {
            Removal::Prefix(span) => format!("prefix {:?}", text(span)),
            Removal::Statement(span) => format!("statement {:?}", text(span)),
            Removal::Item { items, index, all } => format!(
                "item {index} of {:?}, all {:?}",
                items.iter().map(text).collect::<Vec<_>>(),
                all.as_ref().map(text)
            ),
            Removal::Import {
                statement, index, ..
            } => format!("import {index} of {:?}", text(statement)),
        }
    }

    fn export_removals(path: &str, source: &str) -> Vec<(String, String)> {
        let found = facts_at(path, source);
        found
            .export_removals
            .iter()
            .map(|(path, r)| (path.join("."), removal(source, r)))
            .collect()
    }

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn commonjs_exports_know_how_to_be_removed() {
        assert_eq!(
            export_removals(
                "a.js",
                "exports.a = 1;\nmodule.exports.b = () => {};\nuse(exports.c = 3);\n\
                 class X { static m() {} }\nexports.X = X;"
            ),
            pairs(&[
                ("X", "statement \"exports.X = X;\""),
                ("X.m", "statement \"static m() {}\""),
                ("a", "statement \"exports.a = 1;\""),
                ("b", "statement \"module.exports.b = () => {};\""),
            ])
        );
        assert_eq!(
            export_removals("a.js", "module.exports = { a, b() {}, ...rest };"),
            pairs(&[
                ("a", "item 0 of [\"a\", \"b() {}\", \"...rest\"], all None"),
                ("b", "item 1 of [\"a\", \"b() {}\", \"...rest\"], all None"),
            ])
        );
        assert_eq!(
            export_removals("a.js", "module.exports = function () {};"),
            pairs(&[("default", "statement \"module.exports = function () {};\"")])
        );
        assert_eq!(
            export_removals("a.ts", "function f() {}\nexport = f;"),
            pairs(&[("default", "statement \"export = f;\"")])
        );
    }

    #[test]
    fn esm_exports_know_how_to_be_removed() {
        assert_eq!(
            export_removals(
                "a.ts",
                "export const a = 1, b = 2;\nexport function f() {}\nexport interface I {}\n\
                 const c = 1;\nexport { c, c as d };\nexport const z = make();"
            ),
            pairs(&[
                ("I", "statement \"export interface I {}\""),
                ("a", "statement \"export const a = 1, b = 2;\""),
                ("b", "statement \"export const a = 1, b = 2;\""),
                (
                    "c",
                    "item 0 of [\"c\", \"c as d\"], all Some(\"export { c, c as d };\")"
                ),
                (
                    "d",
                    "item 1 of [\"c\", \"c as d\"], all Some(\"export { c, c as d };\")"
                ),
                ("f", "statement \"export function f() {}\""),
                ("z", "prefix \"export \""),
            ])
        );
        for (source, expected) in [
            (
                "export default function f() {}",
                Some("statement \"export default function f() {}\""),
            ),
            (
                "export default class C {}",
                Some("statement \"export default class C {}\""),
            ),
            (
                "const a = 1;\nexport default a;",
                Some("statement \"export default a;\""),
            ),
            (
                "export default function () {}",
                Some("statement \"export default function () {}\""),
            ),
            (
                "export default 42;",
                Some("statement \"export default 42;\""),
            ),
            ("export default make();", None),
        ] {
            let found = export_removals("a.js", source);
            let expected: Vec<(String, String)> = expected
                .map(|e| ("default".to_string(), e.to_string()))
                .into_iter()
                .collect();
            assert_eq!(found, expected, "{source}");
        }
    }

    fn import_removals(path: &str, source: &str) -> Vec<(String, String)> {
        let found = facts_at(path, source);
        let mut found: Vec<(String, String)> = found
            .import_removals
            .iter()
            .map(|((specifier, name), r)| (format!("{name} from {specifier}"), removal(source, r)))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn unused_requires_know_how_to_be_removed() {
        assert_eq!(
            import_removals(
                "a.js",
                "const x = require('./x');\nconst { a, b: c } = require('./y');\n\
                 const z = require('./z'), other = 1;\nconst { d, ...rest } = require('./w');\n\
                 const { e } = require('./v'), more = 2;"
            ),
            pairs(&[
                (
                    "a from ./y",
                    "item 0 of [\"a\", \"b: c\"], all Some(\"const { a, b: c } = require('./y');\")"
                ),
                (
                    "c from ./y",
                    "item 1 of [\"a\", \"b: c\"], all Some(\"const { a, b: c } = require('./y');\")"
                ),
                ("e from ./v", "item 0 of [\"e\"], all None"),
                ("x from ./x", "statement \"const x = require('./x');\""),
            ])
        );
    }

    #[test]
    fn the_bindings_of_markup_scripts_are_used_without_a_reference() {
        let source = "import { a, b } from './x';\nconst c = require('./c');\nb();";
        let facts = analyze_markup(source, std::path::Path::new("a.js")).unwrap();
        let x = &facts.requires[0];
        assert!(x.unused.is_empty());
        assert!(x.uses.contains(&Use {
            path: vec!["a".to_string()],
            whole: true
        }));
        assert!(facts.requires[1].unused.is_empty());
        let facts = analyze(source, std::path::Path::new("a.js")).unwrap();
        assert_eq!(facts.requires[0].unused, ["a"]);
    }

    #[test]
    fn unused_imports_know_how_to_be_removed() {
        assert_eq!(
            import_removals(
                "a.ts",
                "import a, { b, c as d } from './x';\nimport * as ns from './y';\n\
                 import e = require('./z');\nimport j from './j.json' with { type: 'json' };"
            ),
            pairs(&[
                (
                    "a from ./x",
                    "import 0 of \"import a, { b, c as d } from './x';\""
                ),
                (
                    "b from ./x",
                    "import 1 of \"import a, { b, c as d } from './x';\""
                ),
                (
                    "d from ./x",
                    "import 2 of \"import a, { b, c as d } from './x';\""
                ),
                ("e from ./z", "statement \"import e = require('./z');\""),
                ("ns from ./y", "import 0 of \"import * as ns from './y';\""),
            ])
        );
    }

    #[test]
    fn ignore_comments_know_where_they_are() {
        let source = "// prunr-ignore-next-line\nexport const a = 1;\n/* prunr-ignore-file */";
        let found = facts(source);
        let comments: Vec<(u32, &str)> = found
            .ignore_comments
            .iter()
            .map(|(line, span)| (*line, &source[span.start as usize..span.end as usize]))
            .collect();
        assert_eq!(
            comments,
            [
                (1, "// prunr-ignore-next-line"),
                (3, "/* prunr-ignore-file */")
            ]
        );
    }

    #[test]
    fn a_jsdoc_import_type_is_a_type_only_require() {
        let found = facts(
            "/**\n * @param {import(\"./types\").Invoice[]} a\n * @param {import( './flags' )} b\n \
             * @param {typeof import(`./c`).default} c\n * @type {import(x)} d\n \
             * @type {import(\"./a\" + b)} e\n * @type {import(\"./open} f\n */\n\
             /* import(\"./block\") */\n// import(\"./line\")\nfunction f(a, b, c, d) {}",
        );
        let requires: Vec<(&str, &[Use], bool, bool)> = found
            .requires
            .iter()
            .map(|r| (r.specifier.as_str(), r.uses.as_slice(), r.type_only, r.esm))
            .collect();
        assert_eq!(
            requires,
            [
                ("./types", &[uses(&["Invoice"], true)][..], true, false),
                ("./flags", &[uses(&[], true)][..], true, false),
                ("./c", &[uses(&["default"], true)][..], true, false),
            ]
        );
        assert_eq!(found.require_lines["./types"], 2);
        assert!(found.requires.iter().all(|r| r.unused.is_empty()));
    }

    fn declarations(source: &str) -> Vec<(String, String)> {
        let found = facts(source);
        found
            .export_declarations
            .iter()
            .map(|(path, span)| {
                let text = &source[span.start as usize..span.end as usize];
                (path.join("."), text.to_string())
            })
            .collect()
    }

    #[test]
    fn an_export_of_a_local_knows_the_declaration_it_would_leave_unused() {
        let source = "/** Doc. */\nconst a = `x`, b = 2;\n\n// Header.\n\nconst c = -1;\n\
                      const d = [1, , 2];\nconst e = Object.freeze({ k: 'v' });\n\
                      const f = Object.freeze(g());\nconst h = delete o.p;\nconst i = `${j}`;\n\
                      const k = { [key]: 1 };\nfunction l() {}\nclass M {}\nconst n = 1;\nuse(n);\n\
                      module.exports = { a, c, d, e, f, h, i, k, l, M, n };";
        assert_eq!(
            declarations(source),
            pairs(&[
                ("M", "class M {}"),
                ("c", "const c = -1;"),
                ("d", "const d = [1, , 2];"),
                ("e", "const e = Object.freeze({ k: 'v' });"),
                ("l", "function l() {}"),
            ]),
            "a declaration of several names, one with side effects and one read elsewhere stay"
        );
        let source = "/** Doc. */\nconst a = 1;\nexports.a = a;";
        assert_eq!(
            declarations(source),
            pairs(&[("a", "/** Doc. */\nconst a = 1;")])
        );
    }

    #[test]
    fn a_syntax_error_fails_the_analysis() {
        let error = analyze("const = ;", std::path::Path::new("index.js")).unwrap_err();
        assert!(!error.is_empty());
    }
}
