//! The project as one module graph: every file is read and parsed once, and its requires are
//! resolved to the files and packages they point to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::facts::{Facts, ReexportKind, Use, analyze};
use crate::resolve::{Resolver, Target};

/// A path relative to the project root, with `/` separators.
pub type FileId = String;

#[derive(Debug)]
pub struct Project {
    /// The packages the repository versions in a `node_modules` folder.
    pub vendored: BTreeSet<String>,
    /// The packages each `package.json` that is not a package of the repository declares (a
    /// test harness with its own dependencies), by its directory: a file under it can import
    /// them, as Node.js looks for packages in every folder above the file.
    pub nested_declared: BTreeMap<String, BTreeSet<String>>,
    /// The entries the root `package.json` declares (each subpath of `exports`, `main`,
    /// `module`), found in the source when they point to a build.
    pub root_entries: BTreeSet<FileId>,
    /// The canonical project root.
    pub root: PathBuf,
    pub files: BTreeMap<FileId, File>,
    /// The packages of the repository: the root one first, then those a monorepo tool declares.
    pub workspaces: Vec<Workspace>,
    /// Test code the plugins name: test files, the tools' configuration and setup files.
    pub test_files: BTreeSet<FileId>,
    /// Production entries the frameworks start from by convention (Next.js routes, NestJS main),
    /// and the entry of each package of the repository that is published.
    pub production_files: BTreeSet<FileId>,
    /// How many files were parsed.
    pub parsed: usize,
}

/// One package of the repository and what it declares and uses outside its files.
#[derive(Debug, Default)]
pub struct Workspace {
    /// Its directory, relative to the root, with `/` separators: empty for the root package.
    pub dir: String,
    /// Its `name`: importing itself by it needs no declaration.
    pub name: Option<String>,
    /// Not `"private": true`: what it exposes is used by whoever installs it.
    pub published: bool,
    /// Packages in `dependencies`: they must be used, and by production code.
    pub dependencies: BTreeSet<String>,
    /// Packages in `devDependencies`: they must be used.
    pub dev_dependencies: BTreeSet<String>,
    /// Every package its `package.json` declares, in any of its dependency fields.
    pub declared_packages: BTreeSet<String>,
    /// Packages its `scripts` run or preload.
    pub script_packages: BTreeSet<String>,
    /// Packages the plugins say the tools load by name (presets, environments, reporters).
    pub test_packages: BTreeSet<String>,
    /// Packages the frameworks load in production without an import.
    pub production_packages: BTreeSet<String>,
    /// The `peerDependencies` of each installed package it can use: whoever uses the package
    /// uses them too.
    pub peers: BTreeMap<String, BTreeSet<String>>,
    /// The binaries its scripts run that only an installed package it does not declare
    /// provides, as `(script, binary, package)`.
    pub undeclared_binaries: BTreeSet<(String, String, String)>,
}

#[derive(Debug, Default)]
pub struct File {
    pub facts: Facts,
    /// Requires that point to project files.
    pub edges: Vec<Edge>,
    /// Packages the file requires.
    pub packages: BTreeSet<String>,
    /// Requires that point nowhere: dynamic ones by their source, missing files by specifier.
    pub unresolved: Vec<String>,
    /// Why the file could not be parsed, if it could not: then it has no facts.
    pub parse_error: Option<String>,
    /// `export ... from` other project files: uses of this file that they forward there.
    pub reexports: Vec<(FileId, ReexportKind)>,
    /// The file requires a Node.js builtin module.
    pub builtins: bool,
    /// The first line that requires each project file this one depends on.
    pub edge_lines: BTreeMap<FileId, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub target: FileId,
    /// What the file uses of the target, in the target's terms: an ESM import of a CommonJS
    /// module has `module.exports` as its default, so the `default` is dropped.
    pub uses: Vec<Use>,
    /// Only types flow through it: TypeScript erases it.
    pub type_only: bool,
}

impl File {
    /// Records where a specifier points; the project file it names, if any.
    fn link(
        &mut self,
        target: Target,
        specifier: &str,
        files: &BTreeMap<PathBuf, FileId>,
    ) -> Option<FileId> {
        match target {
            Target::File(path) => return files.get(&path).cloned(),
            Target::Package(package) => {
                self.packages.insert(package);
            }
            Target::Builtin => self.builtins = true,
            Target::Unresolved => self.unresolved.push(specifier.to_string()),
        }
        None
    }
}

/// A use of a module through an ESM import, made relative to `module.exports` for a CommonJS
/// target: its `default` is `module.exports` itself.
pub fn as_commonjs(used: &Use) -> Use {
    let path = match used.path.split_first() {
        Some((first, rest)) if first == "default" => rest.to_vec(),
        _ => used.path.clone(),
    };
    Use {
        path,
        whole: used.whole,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectError {
    PackageJson(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageJson(reason) => write!(f, "invalid package.json: {reason}"),
        }
    }
}

/// The dependency fields of `package.json`.
#[derive(Default)]
struct Manifest {
    name: Option<String>,
    private: bool,
    main: Option<String>,
    module: Option<String>,
    exports: Option<serde_json::Value>,
    dependencies: BTreeMap<String, serde_json::Value>,
    dev_dependencies: BTreeMap<String, serde_json::Value>,
    peer_dependencies: BTreeMap<String, serde_json::Value>,
    optional_dependencies: BTreeMap<String, serde_json::Value>,
    scripts: BTreeMap<String, String>,
    /// The files of its `bin`: the commands it installs.
    bin: Vec<String>,
    /// The whole `package.json`, for the plugins: tools keep their configuration in it.
    raw: serde_json::Value,
}

impl Manifest {
    /// The `package.json` in `dir` (relative to `root`), or an empty one.
    fn load(root: &Path, dir: &str) -> Result<Self, ProjectError> {
        let path = root.join(dir).join("package.json");
        if !path.is_file() {
            // A folder without package.json is not a package anyone installs.
            return Ok(Self {
                private: true,
                ..Self::default()
            });
        }
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| {
                let at = if dir.is_empty() {
                    String::new()
                } else {
                    format!("{dir}/package.json: ")
                };
                ProjectError::PackageJson(format!("{at}{e}"))
            })?;
        Ok(Self::from_value(&value))
    }

    /// The fields prunr reads, each as far as it has the type npm documents: a field of another
    /// type (`"private": "true"`, a `name` that is a number) is read leniently or left out,
    /// as package managers do, instead of failing the check.
    fn from_value(value: &serde_json::Value) -> Self {
        use serde_json::Value;
        let text = |key: &str| value[key].as_str().map(str::to_string);
        let map = |key: &str| match &value[key] {
            Value::Object(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => BTreeMap::new(),
        };
        let some = |key: &str| Some(value[key].clone()).filter(|value| !value.is_null());
        Self {
            name: text("name"),
            private: matches!(&value["private"], Value::Bool(true))
                || value["private"].as_str() == Some("true"),
            main: text("main"),
            module: text("module"),
            exports: some("exports"),
            dependencies: map("dependencies"),
            dev_dependencies: map("devDependencies"),
            peer_dependencies: map("peerDependencies"),
            optional_dependencies: map("optionalDependencies"),
            scripts: map("scripts")
                .into_iter()
                .filter_map(|(name, script)| Some((name, script.as_str()?.to_string())))
                .collect(),
            bin: match &value["bin"] {
                Value::String(path) => vec![path.clone()],
                Value::Object(bins) => bins
                    .values()
                    .filter_map(|path| path.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            },
            raw: value.clone(),
        }
    }

    /// Every package it declares, in any dependency field.
    fn declared(&self) -> BTreeSet<String> {
        [
            &self.dependencies,
            &self.dev_dependencies,
            &self.peer_dependencies,
            &self.optional_dependencies,
        ]
        .into_iter()
        .flat_map(|field| field.keys().cloned())
        .collect()
    }

    /// Where its entry may be, relative to its directory: what `exports` and `main` say, then
    /// the source a build would compile it from.
    fn entry_candidates(&self) -> Vec<String> {
        let mut entries = self.dot_export().map(targets).unwrap_or_default();
        entries.extend(self.main.iter().chain(&self.module).cloned());
        let mut candidates: Vec<String> = entries.iter().flat_map(|e| from_build(e)).collect();
        // The source of an entry a build writes elsewhere (`./index.js` from `lib/chai.js`).
        candidates.extend(["src/index", "index", "lib/index"].map(str::to_string));
        if let Some(name) = &self.name {
            let name = name.rsplit('/').next().unwrap_or(name);
            candidates.extend([format!("src/{name}"), format!("lib/{name}")]);
        }
        candidates
    }

    /// Whether `exports` maps subpaths (`{ ".": ..., "./lib": ... }`) rather than being the
    /// export of `.` itself (a path, or conditions).
    fn has_subpaths(&self) -> bool {
        matches!(&self.exports, Some(serde_json::Value::Object(map))
            if map.keys().any(|key| key.starts_with('.')))
    }

    /// What `exports` says the `.` subpath is.
    fn dot_export(&self) -> Option<&serde_json::Value> {
        match self.has_subpaths() {
            true => self.exports.as_ref()?.get("."),
            false => self.exports.as_ref(),
        }
    }

    /// For each subpath `exports` declares besides `.` (`./lib/parser`), where it may be.
    fn subpath_candidates(&self) -> Vec<Vec<String>> {
        let Some(serde_json::Value::Object(map)) = &self.exports else {
            return Vec::new();
        };
        map.iter()
            .filter(|(key, _)| key.starts_with("./"))
            .map(|(_, value)| targets(value).iter().flat_map(|t| from_build(t)).collect())
            .collect()
    }
}

/// The files an `exports` value points to, under any condition but `types` (nested conditions
/// and fallback arrays too); `null` blocks a subpath.
fn targets(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(path) if !path.ends_with(".d.ts") => vec![path.clone()],
        serde_json::Value::Object(conditions) => conditions
            .iter()
            .filter(|(condition, _)| *condition != "types")
            .flat_map(|(_, value)| targets(value))
            .collect(),
        serde_json::Value::Array(fallbacks) => fallbacks.iter().flat_map(targets).collect(),
        _ => Vec::new(),
    }
}

/// An entry and, when it is in a build folder (`./dist/cjs/index.js`), the paths of the source
/// it may be compiled from: without the build folder and the folders after it, as they are and
/// under `src/` and `lib/` (`cjs/index.js`, `src/cjs/index.js`, ..., `src/index.js`,
/// `lib/index.js`). The resolver finds the `.ts` of a `.js`.
fn from_build(entry: &str) -> Vec<String> {
    let mut candidates = vec![entry.to_string()];
    let segments: Vec<&str> = entry.trim_start_matches("./").split('/').collect();
    if segments.len() > 1 && BUILD_FOLDERS.contains(&segments[0]) {
        for start in 1..segments.len() {
            let rest = segments[start..].join("/");
            candidates.push(rest.clone());
            candidates.push(format!("src/{rest}"));
            candidates.push(format!("lib/{rest}"));
        }
    }
    candidates
}

/// The files a package declares as its entries: its main one and each subpath of `exports`.
fn package_entries(
    resolver: &Resolver,
    root: &Path,
    dir: &str,
    manifest: &Manifest,
    files: &BTreeMap<PathBuf, FileId>,
) -> BTreeSet<FileId> {
    std::iter::once(manifest.entry_candidates())
        .chain(manifest.subpath_candidates())
        .filter_map(|candidates| package_file(resolver, root, dir, &candidates, files))
        .collect()
}

/// The files of the commands a package's `bin` declares, found in the source when they point to
/// a build.
fn package_bins(
    resolver: &Resolver,
    root: &Path,
    dir: &str,
    manifest: &Manifest,
    files: &BTreeMap<PathBuf, FileId>,
) -> BTreeSet<FileId> {
    manifest
        .bin
        .iter()
        .filter_map(|bin| package_file(resolver, root, dir, &from_build(bin), files))
        .collect()
}

/// The folders a build usually writes to.
pub(crate) const BUILD_FOLDERS: &[&str] = &["dist", "build", "lib", "out"];

/// The file `candidates` (relative to the package at `dir`) point to: the first that exists.
fn package_file(
    resolver: &Resolver,
    root: &Path,
    dir: &str,
    candidates: &[String],
    files: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let manifest = root.join(dir).join("package.json");
    candidates.iter().find_map(|candidate| {
        let relative = format!("./{}", candidate.trim_start_matches("./"));
        match resolver.resolve(&manifest, &relative) {
            Target::File(path) => files.get(&path).cloned(),
            _ => None,
        }
    })
}

/// The binaries each package declares, mapped to the package. An installed package declares
/// those in the `bin` of its `package.json`; one that is not installed (or unreadable) is guessed
/// to declare a binary named after it, without the scope.
fn binaries(installed: &BTreeMap<String, Option<serde_json::Value>>) -> BTreeMap<String, String> {
    let mut binaries = BTreeMap::new();
    for (package, installed) in installed {
        let unscoped = || package.rsplit('/').next().unwrap_or(package).to_string();
        let names = match installed.as_ref().map(|manifest| manifest.get("bin")) {
            Some(Some(serde_json::Value::Object(bins))) => bins.keys().cloned().collect(),
            Some(Some(_)) | None => vec![unscoped()],
            Some(None) => Vec::new(),
        };
        for name in names {
            binaries.insert(name, package.clone());
        }
    }
    binaries
}

/// Every binary the packages installed in the `node_modules` of `dirs` provide, with its
/// package: the first of `dirs` that has a binary wins.
fn installed_binaries(dirs: &[PathBuf]) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    let entries = |dir: &Path| -> Vec<PathBuf> {
        let read = std::fs::read_dir(dir).into_iter().flatten().flatten();
        read.map(|entry| entry.path()).collect()
    };
    for dir in dirs.iter().rev() {
        let mut packages = Vec::new();
        for path in entries(&dir.join("node_modules")) {
            match path.file_name().and_then(|name| name.to_str()) {
                Some(name) if name.starts_with('@') => packages.extend(entries(&path)),
                _ => packages.push(path),
            }
        }
        for path in packages {
            let Ok(text) = std::fs::read_to_string(path.join("package.json")) else {
                continue;
            };
            let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            let Some(package) = manifest["name"].as_str() else {
                continue;
            };
            let installed = BTreeMap::from([(package.to_string(), Some(manifest.clone()))]);
            found.extend(binaries(&installed));
        }
    }
    found
}

/// The `package.json` of each package as installed in the `node_modules` of the first of `dirs`
/// that has it; `None` when it is not installed or cannot be read.
fn installed(
    dirs: &[PathBuf],
    packages: &BTreeSet<String>,
) -> BTreeMap<String, Option<serde_json::Value>> {
    packages
        .iter()
        .map(|package| {
            let manifest = dirs.iter().find_map(|dir| {
                let manifest = dir.join("node_modules").join(package).join("package.json");
                std::fs::read_to_string(manifest)
                    .ok()
                    .and_then(|text| serde_json::from_str(&text).ok())
            });
            (package.clone(), manifest)
        })
        .collect()
}

/// The `peerDependencies` of each installed package that has any.
fn peers(
    installed: &BTreeMap<String, Option<serde_json::Value>>,
) -> BTreeMap<String, BTreeSet<String>> {
    installed
        .iter()
        .filter_map(|(package, manifest)| {
            let serde_json::Value::Object(peers) = &manifest.as_ref()?["peerDependencies"] else {
                return None;
            };
            Some((package.clone(), peers.keys().cloned().collect()))
        })
        .collect()
}

/// The packages each `package.json` under `root` that is not one of the repository's packages
/// (`dirs`) declares, by its directory. One that is not valid JSON declares nothing.
fn nested_manifests(root: &Path, dirs: &[String]) -> BTreeMap<String, BTreeSet<String>> {
    ignore::WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build()
        .flatten()
        .filter(|entry| entry.file_name() == "package.json")
        .filter_map(|entry| {
            let dir = file_id(root, entry.path().parent()?)?;
            let manifest = Manifest::load(root, &dir).ok()?;
            (!dirs.contains(&dir)).then(|| (dir, manifest.declared()))
        })
        .collect()
}

/// `path` relative to `root`, with `/` separators.
fn file_id(root: &Path, path: &Path) -> Option<FileId> {
    let relative = path.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

/// The extensions of the source files prunr reads.
const EXTENSIONS: &[&str] = &["js", "cjs", "mjs", "jsx", "ts", "cts", "mts", "tsx"];

/// The extensions of the source files prunr reads, for the plugins.
pub(crate) fn source_extensions() -> &'static [&'static str] {
    EXTENSIONS
}

/// A TypeScript declaration file: it only declares types, so it is not part of the graph.
fn is_declaration(path: &Path) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    [".d.ts", ".d.cts", ".d.mts"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// Every source file under `root`, without `node_modules`, declarations and ignored files.
/// The files git tracks under `root`, relative to it; none outside a repository.
fn tracked(root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| output.stdout)
        .unwrap_or_default();
    output
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .collect()
}

/// The packages the repository keeps in a `node_modules` folder it versions (a test helper in
/// `test/node_modules/`): part of the project, not dependencies to declare.
fn vendored(root: &Path) -> BTreeSet<String> {
    tracked(root)
        .iter()
        .filter_map(|path| path.rsplit_once("node_modules/").map(|(_, inside)| inside))
        .map(|inside| crate::resolve::package_name(inside).to_string())
        .collect()
}

/// The source files under `root`, and those with an extension a plugin compiles.
fn source_files(root: &Path, compiled: &[&str]) -> BTreeMap<PathBuf, FileId> {
    let walker = ignore::WalkBuilder::new(root)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != "node_modules")
        .build();
    let walked = walker.flatten().map(|entry| entry.into_path());
    // A file git tracks is part of the project even when an ignore pattern matches it
    // (`mocha.js` in .gitignore, for a bundle at the root, also matches `bin/mocha.js`).
    let tracked = tracked(root).into_iter().map(|path| root.join(path));
    let mut files = BTreeMap::new();
    for path in walked.chain(tracked) {
        let source = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|extension| {
                EXTENSIONS.contains(&extension) || compiled.contains(&extension)
            });
        let installed = path
            .components()
            .any(|part| part.as_os_str() == "node_modules");
        if source && !installed && !is_declaration(&path) && path.is_file() {
            let id = file_id(root, &path).expect("walked files are under the root");
            files.insert(path, id);
        }
    }
    files
}

/// Analyzes one file: a source, or what the compiler of its extension makes of it. The imports
/// of a compiled file are used: the markup around its scripts can use them.
fn analyze_file(
    source: &str,
    path: &Path,
    compilers: &BTreeMap<String, crate::plugins::Compiler>,
) -> Result<Facts, String> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(compiler) = compilers.get(extension) else {
        return analyze(source, path);
    };
    let compiled = (compiler.0)(source);
    let language = match compiled.typescript {
        true => "ts",
        false => "js",
    };
    crate::facts::analyze_markup(&compiled.source, &path.with_extension(language))
}

/// The source file of the project a script names, relative to `dir`, with or without its
/// extension.
fn script_file(dir: &Path, argument: &str, paths: &BTreeMap<PathBuf, FileId>) -> Option<FileId> {
    let path = dir.join(argument);
    let with_extension = EXTENSIONS.iter().map(|extension| {
        let mut name = path.clone().into_os_string();
        name.push(format!(".{extension}"));
        PathBuf::from(name)
    });
    std::iter::once(path.clone())
        .chain(with_extension)
        .find(|path| path.is_file())
        .and_then(|path| paths.get(&crate::resolve::canonical(&path)).cloned())
}

/// The files other than `from` that a require pattern in `from` can load: those the glob
/// matches with or without their extension, and directories through their index file.
fn matching<'f>(from: &FileId, glob: &str, files: impl Iterator<Item = &'f FileId>) -> Vec<FileId> {
    let mut segments: Vec<&str> = from.split('/').collect();
    segments.pop();
    for segment in glob.split('/') {
        match segment {
            "." => {}
            ".." if segments.pop().is_none() => return Vec::new(),
            ".." => {}
            segment => segments.push(segment),
        }
    }
    let glob = globset::GlobBuilder::new(&segments.join("/"))
        .literal_separator(true)
        .build()
        .expect("a require pattern escapes its literal text")
        .compile_matcher();
    let mut targets: Vec<FileId> = files
        .filter(|id| *id != from)
        .filter(|id| {
            let stem = id.rsplit_once('.').map_or(id.as_str(), |(stem, _)| stem);
            let directory = stem.strip_suffix("/index");
            glob.is_match(id) || glob.is_match(stem) || directory.is_some_and(|d| glob.is_match(d))
        })
        .cloned()
        .collect();
    targets.sort();
    targets
}

/// The files the globs a plugin gives match. An invalid glob matches nothing.
fn plugin_files(patterns: &[String], files: &BTreeMap<FileId, File>) -> BTreeSet<FileId> {
    patterns
        .iter()
        .filter_map(|pattern| glob(pattern).ok())
        .flat_map(|glob| files.keys().filter(move |id| glob.is_match(id)))
        .cloned()
        .collect()
}

/// The index in `dirs` of the package `file` belongs to: the one whose directory is the longest
/// prefix of its path (the root, `""`, when none is).
fn workspace_index(dirs: &[&str], file: &str) -> usize {
    dirs.iter()
        .enumerate()
        .filter(|(_, dir)| {
            dir.is_empty()
                || file
                    .strip_prefix(**dir)
                    .is_some_and(|rest| rest.starts_with('/'))
        })
        .max_by_key(|(_, dir)| dir.len())
        .map_or(0, |(index, _)| index)
}

/// A glob over file ids, where `*` does not cross `/` and `**` does.
fn glob(pattern: &str) -> Result<globset::GlobMatcher, globset::Error> {
    Ok(globset::GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()?
        .compile_matcher())
}

impl Project {
    /// Reads every `.js` and `.cjs` file under `root`, skipping `node_modules` and what
    /// `.gitignore` ignores.
    pub fn load(root: &Path) -> Result<Self, ProjectError> {
        let root = crate::resolve::canonical(root);
        let dirs: Vec<String> = std::iter::once(String::new())
            .chain(crate::workspaces::discover(&root))
            .collect();
        let manifests = dirs
            .iter()
            .map(|dir| Manifest::load(&root, dir))
            .collect::<Result<Vec<_>, _>>()?;
        // A package of the repository imported by its name: `@app/shared` or `@app/shared/x`.
        let names: BTreeMap<&str, usize> = manifests
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(index, manifest)| Some((manifest.name.as_deref()?, index)))
            .collect();
        let dir_names: Vec<&str> = dirs.iter().map(String::as_str).collect();
        let resolver = Resolver::default();
        let root_declared = manifests[0].declared();
        // What the root declares is available to every package (hoisting, shared tools).
        let visible: Vec<BTreeSet<String>> = manifests
            .iter()
            .map(|manifest| manifest.declared().union(&root_declared).cloned().collect())
            .collect();
        let contributions: Vec<crate::plugins::Contribution> = dirs
            .iter()
            .zip(&manifests)
            .zip(&visible)
            .map(|((dir, manifest), visible)| {
                crate::plugins::contribution(&crate::plugins::Context {
                    root: &root.join(dir),
                    declared: visible,
                    manifest: &manifest.raw,
                })
            })
            .collect();
        // The files of every package that a plugin compiles (`.vue`) are sources too.
        // A package's plugins compile its own files only: the `.svelte` routes of a SvelteKit app
        // are not sources because a sibling package uses Svelte.
        let compilers_of = |id: &str| &contributions[workspace_index(&dir_names, id)].compilers;
        let compiled: BTreeSet<&str> = contributions
            .iter()
            .flat_map(|contribution| contribution.compilers.keys())
            .map(String::as_str)
            .collect();
        let mut paths = source_files(&root, &compiled.into_iter().collect::<Vec<_>>());
        paths.retain(|path, id| {
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default();
            EXTENSIONS.contains(&extension) || compilers_of(id).contains_key(extension)
        });
        // Each file is read and analyzed on its own, in parallel; the results keep the order of
        // `paths`.
        use rayon::prelude::*;
        let results: Vec<(FileId, Result<Facts, String>)> = paths
            .par_iter()
            .map(|(path, id)| {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                (id.clone(), analyze_file(&source, path, compilers_of(id)))
            })
            .collect();
        // A file that does not parse (a fixture broken on purpose, a syntax prunr does not read)
        // is in the graph without facts; the checks warn when something reaches it.
        let mut analyzed = BTreeMap::new();
        let mut parse_errors = BTreeMap::new();
        for (id, facts) in results {
            let facts = facts.unwrap_or_else(|message| {
                parse_errors.insert(id.clone(), message);
                Facts::default()
            });
            analyzed.insert(id, facts);
        }
        let esm: BTreeSet<FileId> = analyzed
            .iter()
            .filter(|(_, facts)| facts.esm)
            .map(|(id, _)| id.clone())
            .collect();
        let workspace_file = |specifier: &str| -> Option<(String, FileId)> {
            let name = crate::resolve::package_name(specifier);
            let &index = names.get(name)?;
            let subpath = &specifier[name.len()..];
            let candidates = if subpath.is_empty() {
                manifests[index].entry_candidates()
            } else {
                vec![format!(".{subpath}"), format!("src{subpath}")]
            };
            let file = package_file(&resolver, &root, &dirs[index], &candidates, &paths)?;
            Some((name.to_string(), file))
        };
        // Where a specifier points: a project file, else what `link` records.
        // A name a tool maps (Jest's `moduleNameMapper` and `__mocks__`, Vite's `resolve.alias`):
        // to a file of the package, as an absolute path, or to another specifier.
        let aliased = |from: &Path, specifier: &str| -> Option<String> {
            let index = workspace_index(&dir_names, paths.get(from)?);
            let target = contributions[index]
                .aliases
                .iter()
                .find_map(|alias| alias.apply(specifier))?;
            Some(match target.strip_prefix("./") {
                Some(path) => root
                    .join(&dirs[index])
                    .join(path)
                    .to_string_lossy()
                    .into_owned(),
                None => target,
            })
        };
        let target_of = |file: &mut File, from: &Path, specifier: &str| {
            let aliased = aliased(from, specifier);
            let specifier = aliased.as_deref().unwrap_or(specifier);
            match workspace_file(specifier) {
                Some((package, target)) => {
                    file.packages.insert(package);
                    Some(target)
                }
                None => {
                    let mut target = resolver.resolve(from, specifier);
                    if let Target::File(path) = &target
                        && crate::stylesheets::is_stylesheet(path)
                    {
                        file.packages.extend(crate::stylesheets::packages(path));
                    }
                    if target == Target::Unresolved && specifier.starts_with('/') {
                        // A bundler's absolute specifier: from its public directory or root.
                        let id = &paths[from];
                        let index = workspace_index(&dir_names, id);
                        let package = root.join(&dirs[index]);
                        let roots = &contributions[index].absolute_roots;
                        if let Some(found) = roots
                            .iter()
                            .map(|dir| package.join(dir).join(&specifier[1..]))
                            .map(|path| resolver.resolve(from, &path.to_string_lossy()))
                            .find(|found| *found != Target::Unresolved)
                        {
                            target = found;
                        }
                    }
                    // A folder with a package.json whose entry points to a build that does not
                    // exist (`require("..")` from a package's tests): its entry in the source.
                    if target == Target::Unresolved
                        && specifier.starts_with('.')
                        && let Some(dir) = from.parent().map(|dir| dir.join(specifier))
                        && dir.join("package.json").is_file()
                        && let Some(dir) = file_id(&root, &crate::resolve::canonical(&dir))
                        && let Ok(manifest) = Manifest::load(&root, &dir)
                        && let Some(id) = package_file(
                            &resolver,
                            &root,
                            &dir,
                            &manifest.entry_candidates(),
                            &paths,
                        )
                    {
                        target = Target::File(root.join(id));
                    }
                    file.link(target, specifier, &paths)
                }
            }
        };
        // Each file is linked on its own, in parallel: the resolver is shared and caches.
        let jobs: Vec<(&PathBuf, &FileId, Facts)> = paths
            .iter()
            .map(|(path, id)| {
                let facts = analyzed.remove(id).expect("every file was analyzed");
                (path, id, facts)
            })
            .collect();
        let linked: Vec<(FileId, File)> = jobs
            .into_par_iter()
            .map(|(path, id, facts)| {
                let mut file = File::default();
                for require in &facts.requires {
                    if let Some(target) = target_of(&mut file, path, &require.specifier) {
                        file.edge_lines
                            .entry(target.clone())
                            .or_insert(facts.require_lines[&require.specifier]);
                        let uses = if require.esm && !esm.contains(&target) {
                            require.uses.iter().map(as_commonjs).collect()
                        } else {
                            require.uses.clone()
                        };
                        file.edges.push(Edge {
                            target,
                            uses,
                            type_only: require.type_only,
                        });
                    }
                }
                for reexport in &facts.reexports {
                    if let Some(target) = target_of(&mut file, path, &reexport.specifier) {
                        file.edge_lines
                            .entry(target.clone())
                            .or_insert(facts.require_lines[&reexport.specifier]);
                        file.edges.push(Edge {
                            target: target.clone(),
                            uses: Vec::new(),
                            type_only: reexport.type_only,
                        });
                        file.reexports.push((target, reexport.kind.clone()));
                    }
                }
                for pattern in &facts.patterns {
                    let targets = matching(id, &pattern.glob, paths.values());
                    if targets.is_empty() {
                        file.unresolved.push(pattern.source.clone());
                    }
                    for target in targets {
                        file.edge_lines
                            .entry(target.clone())
                            .or_insert(facts.require_lines[&pattern.source]);
                        file.edges.push(Edge {
                            target,
                            uses: vec![Use {
                                path: vec![],
                                whole: true,
                            }],
                            type_only: false,
                        });
                    }
                }
                file.unresolved
                    .extend(facts.dynamic_requires.iter().cloned());
                file.parse_error = parse_errors.get(id).cloned();
                file.facts = facts;
                (id.clone(), file)
            })
            .collect();
        let files: BTreeMap<FileId, File> = linked.into_iter().collect();
        let mut workspaces = Vec::new();
        let mut production_files = BTreeSet::new();
        let mut test_files = BTreeSet::new();
        for (index, ((dir, manifest), plugins)) in
            dirs.iter().zip(&manifests).zip(contributions).enumerate()
        {
            let package = root.join(dir);
            let declared_packages = manifest.declared();
            let installed = installed(&[package.clone(), root.clone()], &visible[index]);
            let binaries = binaries(&installed);
            // A binary no declared package provides, but one in `node_modules` does: it is
            // there by chance, as another package's dependency. One nothing provides is the
            // system's (`git`, `docker`).
            let unknown: Vec<(&String, String)> = manifest
                .scripts
                .iter()
                .flat_map(|(name, script)| {
                    let binaries = crate::scripts::binaries(script).into_iter();
                    binaries.map(move |binary| (name, binary))
                })
                .filter(|(_, binary)| !binaries.contains_key(binary))
                .collect();
            let everything = match unknown.is_empty() {
                true => BTreeMap::new(),
                false => installed_binaries(&[package.clone(), root.clone()]),
            };
            let undeclared_binaries: BTreeSet<(String, String, String)> = unknown
                .into_iter()
                .filter_map(|(name, binary)| {
                    let package = everything.get(&binary)?.clone();
                    Some((name.clone(), binary, package))
                })
                .filter(|(_, _, package)| {
                    !manifests.iter().any(|m| m.name.as_ref() == Some(package))
                })
                .collect();
            // The scripts of `package.json`, and the commands the tools run (Playwright's web
            // servers), which run test code.
            let scripts: Vec<(bool, &String)> = manifest
                .scripts
                .iter()
                .map(|(name, script)| (name.starts_with("test"), script))
                .chain(plugins.scripts.iter().map(|script| (true, script)))
                .collect();
            let script_packages = scripts
                .iter()
                .flat_map(|(_, script)| crate::scripts::packages(script, &binaries))
                .collect();
            let in_package = |globs: &[String]| -> Vec<String> {
                globs
                    .iter()
                    .map(|glob| match dir.as_str() {
                        "" => glob.clone(),
                        dir => format!("{dir}/{glob}"),
                    })
                    .collect()
            };
            production_files.extend(plugin_files(&in_package(&plugins.production), &files));
            for (test, script) in scripts {
                let entries = if test {
                    &mut test_files
                } else {
                    &mut production_files
                };
                for run in crate::scripts::files(script) {
                    let dir = match &run.package {
                        None => dir.as_str(),
                        Some(name) => match names.get(name.as_str()) {
                            Some(&index) => dirs[index].as_str(),
                            None => continue,
                        },
                    };
                    let base = root.join(dir);
                    let file = |argument: &String| script_file(&base, argument, &paths);
                    entries.extend(run.preloads.iter().filter_map(file));
                    entries.extend(run.candidates.iter().find_map(file));
                }
            }
            test_files.extend(plugin_files(&in_package(&plugins.tests), &files));
            for file in &plugins.files {
                if let Target::File(path) = resolver.resolve(&package.join("package.json"), file)
                    && let Some(id) = paths.get(&path)
                {
                    test_files.insert(id.clone());
                }
            }
            // A published package's entry is its interface to whoever installs it.
            // A published package's entries are its interface to whoever installs it, and the
            // commands in `bin` run whether it is published or not.
            if !manifest.private {
                production_files.extend(package_entries(&resolver, &root, dir, manifest, &paths));
            }
            production_files.extend(package_bins(&resolver, &root, dir, manifest, &paths));
            workspaces.push(Workspace {
                dir: dir.clone(),
                name: manifest.name.clone(),
                published: !manifest.private,
                dependencies: manifest.dependencies.keys().cloned().collect(),
                dev_dependencies: manifest.dev_dependencies.keys().cloned().collect(),
                declared_packages,
                script_packages,
                test_packages: plugins.packages,
                production_packages: plugins.production_packages,
                peers: peers(&installed),
                undeclared_binaries,
            });
        }
        let nested_declared = nested_manifests(&root, &dirs);
        let mut root_entries = package_entries(&resolver, &root, "", &manifests[0], &paths);
        root_entries.extend(package_bins(&resolver, &root, "", &manifests[0], &paths));
        Ok(Self {
            vendored: vendored(&root),
            nested_declared,
            root_entries,
            root,
            parsed: files.len(),
            files,
            workspaces,
            test_files,
            production_files,
        })
    }

    /// The index in `workspaces` of the package `file` belongs to: the one whose directory is
    /// the longest prefix of its path.
    pub fn workspace_of(&self, file: &str) -> usize {
        let dirs: Vec<&str> = self.workspaces.iter().map(|w| w.dir.as_str()).collect();
        workspace_index(&dirs, file)
    }

    /// Leaves `packages` out of every dependency check.
    pub fn ignore_packages(&mut self, packages: &[String]) {
        for package in packages {
            for workspace in &mut self.workspaces {
                workspace.dependencies.remove(package);
                workspace.dev_dependencies.remove(package);
                workspace.declared_packages.remove(package);
            }
            for file in self.files.values_mut() {
                file.packages.remove(package);
            }
        }
    }

    /// The files that entry paths or globs (relative to the root) name. Each must name at least
    /// one project file.
    pub fn entries(&self, patterns: &[String]) -> Result<BTreeSet<FileId>, String> {
        let mut entries = BTreeSet::new();
        for pattern in patterns {
            let matched = self.entry(pattern)?;
            if matched.is_empty() {
                return Err(format!("entry matches no source file: {pattern}"));
            }
            entries.extend(matched);
        }
        Ok(entries)
    }

    fn entry(&self, pattern: &str) -> Result<Vec<FileId>, String> {
        if !pattern.contains(['*', '?', '[', '{']) {
            let id = dunce::canonicalize(self.root.join(pattern))
                .ok()
                .and_then(|path| file_id(&self.root, &path));
            return Ok(id
                .into_iter()
                .filter(|id| self.files.contains_key(id))
                .collect());
        }
        let glob = glob(pattern).map_err(|e| format!("invalid entry glob {pattern}: {e}"))?;
        Ok(self
            .files
            .keys()
            .filter(|id| glob.is_match(id))
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        dir
    }

    fn ids(project: &Project) -> Vec<&str> {
        project.files.keys().map(String::as_str).collect()
    }

    #[test]
    fn it_reads_javascript_and_typescript_files_but_not_declarations() {
        let dir = project(&[
            ("index.js", ""),
            ("lib/a.cjs", ""),
            ("lib/b.mjs", ""),
            ("lib/c.jsx", ""),
            ("lib/d.ts", ""),
            ("lib/e.cts", ""),
            ("lib/f.mts", ""),
            ("lib/g.tsx", ""),
            ("lib/types.d.ts", ""),
            ("lib/types.d.mts", ""),
            ("lib/types.d.cts", ""),
            ("data.json", "{}"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            ids(&project),
            [
                "index.js",
                "lib/a.cjs",
                "lib/b.mjs",
                "lib/c.jsx",
                "lib/d.ts",
                "lib/e.cts",
                "lib/f.mts",
                "lib/g.tsx"
            ]
        );
    }

    fn edge(target: &str, uses: Vec<Use>) -> Edge {
        Edge {
            target: target.into(),
            uses,
            type_only: false,
        }
    }

    fn used(path: &[&str], whole: bool) -> Use {
        Use {
            path: path.iter().map(|s| s.to_string()).collect(),
            whole,
        }
    }

    #[test]
    fn importing_commonjs_uses_module_exports_as_the_default() {
        let dir = project(&[
            (
                "index.mjs",
                r#"import a from "./a.cjs"; import { b } from "./a.cjs"; import * as ns from "./a.cjs";
                   import e from "./e.mjs";
                   a.x(); b(); ns.default.y; ns.z; e.w;"#,
            ),
            ("a.cjs", "exports.x = 1;"),
            ("e.mjs", "export default {};"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.files["index.mjs"].edges,
            [
                edge("a.cjs", vec![used(&[], false), used(&["x"], true)]),
                edge("a.cjs", vec![used(&["b"], false)]),
                edge("a.cjs", vec![used(&[], true), used(&["z"], true)]),
                edge(
                    "e.mjs",
                    vec![used(&["default"], false), used(&["default", "w"], true)]
                ),
            ]
        );
    }

    #[test]
    fn an_import_of_types_only_is_a_type_only_edge() {
        let dir = project(&[
            ("index.ts", r#"import type { T } from "./t"; let x: T;"#),
            ("t.ts", "export type T = 1;"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.files["index.ts"].edges[0].type_only);
    }

    #[test]
    fn reexports_are_edges_that_forward_uses() {
        let dir = project(&[
            (
                "index.ts",
                r#"export { a as b } from "./a"; export type * from "./t"; export * from "pkg";
                   export * from "./missing";"#,
            ),
            ("a.ts", "export const a = 1;"),
            ("t.ts", "export type T = 1;"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["index.ts"];
        assert_eq!(
            index.reexports,
            [
                (
                    "a.ts".to_string(),
                    ReexportKind::Named {
                        imported: "a".into(),
                        exported: "b".into()
                    }
                ),
                ("t.ts".to_string(), ReexportKind::All),
            ]
        );
        let edges: Vec<(&str, bool)> = index
            .edges
            .iter()
            .map(|edge| (edge.target.as_str(), edge.type_only))
            .collect();
        assert_eq!(edges, [("a.ts", false), ("t.ts", true)]);
        assert!(index.edges.iter().all(|edge| edge.uses.is_empty()));
        assert_eq!(index.packages, BTreeSet::from(["pkg".to_string()]));
        assert_eq!(index.unresolved, ["./missing"]);
    }

    #[test]
    fn a_file_that_requires_a_builtin_says_so() {
        let dir = project(&[("a.js", r#"require("fs");"#), ("b.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.files["a.js"].builtins);
        assert!(!project.files["b.js"].builtins);
    }

    #[test]
    fn it_skips_node_modules_and_gitignored_files() {
        let dir = project(&[
            ("index.js", ""),
            ("node_modules/pkg/index.js", ""),
            (".gitignore", "dist/\n"),
            ("dist/bundle.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(ids(&project), ["index.js"]);
    }

    #[test]
    fn each_file_is_parsed_once_however_often_it_is_required() {
        let dir = project(&[
            (
                "index.js",
                r#"require("./shared"); require("./a"); require("./b");"#,
            ),
            ("a.js", r#"require("./shared");"#),
            ("b.js", r#"require("./shared"); require("./shared");"#),
            ("shared.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(project.parsed, 4);
        assert_eq!(project.files.len(), 4);
    }

    #[test]
    fn requires_become_edges_packages_or_unresolved() {
        let dir = project(&[
            (
                "index.js",
                r#"const { a } = require("./lib/a");
                   a();
                   require("./data.json");
                   require("lodash/fp");
                   require("fs");
                   require("./missing");
                   require(name);"#,
            ),
            ("lib/a.js", "exports.a = 1;"),
            ("data.json", "{}"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["index.js"];
        assert_eq!(index.edges, [edge("lib/a.js", vec![used(&["a"], false)])]);
        assert_eq!(index.packages, BTreeSet::from(["lodash".to_string()]));
        assert_eq!(index.unresolved, ["./missing", "require(name)"]);
    }

    fn whole() -> Vec<Use> {
        vec![Use {
            path: vec![],
            whole: true,
        }]
    }

    fn targets(file: &File) -> Vec<&str> {
        file.edges.iter().map(|edge| edge.target.as_str()).collect()
    }

    #[test]
    fn a_pattern_uses_every_file_it_matches_whole() {
        let dir = project(&[
            (
                "lib/index.js",
                "require(`./${a}`); require(`../jobs/${a}.job.js`); require(`./sub/${a}`);",
            ),
            ("lib/a.js", ""),
            ("lib/sub/index.js", ""),
            ("lib/sub/deep/b.js", ""),
            ("jobs/daily.job.js", ""),
            ("jobs/daily.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["lib/index.js"];
        assert_eq!(
            targets(index),
            [
                "lib/a.js",
                "lib/sub/index.js",
                "jobs/daily.job.js",
                "lib/sub/index.js"
            ],
            "a file never matches its own pattern, and a directory matches its index"
        );
        assert!(index.edges.iter().all(|edge| edge.uses == whole()));
        assert!(index.unresolved.is_empty(), "{:?}", index.unresolved);
    }

    #[test]
    fn the_files_scripts_run_are_entries_of_their_kind_and_package() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "workspaces": ["tools"],
                    "scripts": {
                        "start": "node -r ./setup.js server.js",
                        "test:e2e": "node --test e2e.js",
                        "gen": "pnpm --filter tools exec tsx gen",
                        "other": "pnpm --filter elsewhere exec tsx x.js"
                    }
                }"#,
            ),
            ("tools/package.json", r#"{ "name": "tools" }"#),
            ("tools/gen.ts", ""),
            ("setup.js", ""),
            ("server.js", ""),
            ("e2e.js", ""),
            ("x.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.production_files.iter().collect::<Vec<_>>(),
            ["server.js", "setup.js", "tools/gen.ts"]
        );
        assert_eq!(project.test_files.iter().collect::<Vec<_>>(), ["e2e.js"]);
    }

    #[test]
    fn an_absolute_specifier_falls_back_to_the_roots_a_bundler_declares() {
        let dir = project(&[
            ("package.json", r#"{ "devDependencies": { "vite": "7" } }"#),
            ("public/logo.svg", "<svg/>"),
            ("lib.js", "export const a = 1;"),
            (
                "index.js",
                r#"import "/logo.svg"; import "/lib.js"; import "/gone.svg";"#,
            ),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let index = &project.files["index.js"];
        assert_eq!(index.unresolved, ["/gone.svg"]);
        assert_eq!(index.edges.len(), 1);
    }

    #[test]
    fn a_package_that_imports_itself_by_name_declares_itself() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "name": "self", "exports": "./dist/index.js", "workspaces": ["packages/*"] }"#,
            ),
            ("packages/a/package.json", r#"{ "name": "@x/a" }"#),
            ("packages/a/index.js", r#"require("@x/a/package.json");"#),
            ("index.js", r#"require("self");"#),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let entries = project
            .entries(&["index.js".to_string(), "packages/a/index.js".to_string()])
            .unwrap();
        let findings = crate::checks::check(&project, &entries, &BTreeSet::new());
        assert_eq!(findings, []);
    }

    #[test]
    fn whoever_uses_a_package_uses_its_peers() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "dependencies": { "host": "1", "peer": "1", "deep": "1", "plain": "1" } }"#,
            ),
            (
                "node_modules/host/package.json",
                r#"{ "peerDependencies": { "peer": "1" } }"#,
            ),
            (
                "node_modules/peer/package.json",
                r#"{ "peerDependencies": { "deep": "1" } }"#,
            ),
            (
                "node_modules/deep/package.json",
                r#"{ "peerDependencies": { "host": "1" } }"#,
            ),
            ("node_modules/plain/package.json", "{}"),
            ("index.js", r#"require("host");"#),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.workspaces[0].peers.keys().collect::<Vec<_>>(),
            ["deep", "host", "peer"]
        );
        let entries = project.entries(&["index.js".to_string()]).unwrap();
        let findings = crate::checks::check(&project, &entries, &BTreeSet::new());
        assert_eq!(
            findings,
            [crate::checks::Finding::UnusedDependency {
                package: "plain".into(),
                workspace: None
            }]
        );
    }

    #[test]
    fn a_pattern_that_matches_nothing_is_unresolved() {
        let dir = project(&[(
            "index.js",
            "require(`./nope/${a}`); require(`../outside/${a}`); require(`./${a}`);",
        )]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.files["index.js"].unresolved,
            [
                "require(`./nope/${a}`)",
                "require(`../outside/${a}`)",
                "require(`./${a}`)"
            ]
        );
    }

    #[test]
    fn package_json_declares_the_packages() {
        let dir = project(&[(
            "package.json",
            r#"{
                "dependencies": { "a": "1" },
                "devDependencies": { "b": "1" },
                "peerDependencies": { "c": "1" },
                "optionalDependencies": { "d": "1" }
            }"#,
        )]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.workspaces[0].dependencies,
            BTreeSet::from(["a".into()])
        );
        assert_eq!(
            project.workspaces[0].dev_dependencies,
            BTreeSet::from(["b".into()])
        );
        assert_eq!(
            project.workspaces[0].declared_packages,
            BTreeSet::from(["a".into(), "b".into(), "c".into(), "d".into()])
        );
    }

    #[test]
    fn the_scripts_use_the_packages_whose_binaries_they_run() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "scripts": {
                        "a": "tsc && tool && plain && solo && broken && missing",
                        "b": "node -r undeclared/register x.js"
                    },
                    "devDependencies": {
                        "typescript": "1", "@scope/tool": "1", "plain": "1",
                        "solo-pkg": "1", "broken": "1", "missing": "1", "unused": "1"
                    }
                }"#,
            ),
            (
                "node_modules/typescript/package.json",
                r#"{ "bin": { "tsc": "bin/tsc" } }"#,
            ),
            (
                "node_modules/@scope/tool/package.json",
                r#"{ "bin": "cli.js" }"#,
            ),
            ("node_modules/plain/package.json", "{}"),
            (
                "node_modules/solo-pkg/package.json",
                r#"{ "bin": { "solo": "cli.js" } }"#,
            ),
            ("node_modules/broken/package.json", "{ nope"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.workspaces[0].script_packages,
            BTreeSet::from([
                "@scope/tool".into(),
                "broken".into(),
                "missing".into(),
                "solo-pkg".into(),
                "typescript".into(),
                "undeclared".into(),
            ]),
            "a package without a bin field runs nothing; one not installed is guessed by name"
        );
    }

    #[test]
    fn the_paths_a_config_and_its_local_imports_write_are_edges() {
        let dir = project(&[
            ("package.json", r#"{ "devDependencies": { "vitest": "3" } }"#),
            (
                "vitest.config.ts",
                "import { s } from './tools/s'; import x from 'vitest'; export default () => s;",
            ),
            (
                "tools/s.ts",
                "export const s = ['./kit/setup.ts', './gone.ts', '../outside.ts', 'kit/x.ts'];",
            ),
            ("kit/setup.ts", ""),
            ("src/a.ts", "export const p = '../kit/setup.ts';"),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let edges = |id: &str| -> Vec<&str> {
            let file = &project.files[id];
            file.edges.iter().map(|edge| edge.target.as_str()).collect()
        };
        assert_eq!(edges("tools/s.ts"), ["kit/setup.ts"]);
        assert_eq!(project.files["tools/s.ts"].edge_lines["kit/setup.ts"], 1);
        assert!(edges("src/a.ts").is_empty());
    }

    #[test]
    fn the_test_runners_name_test_files_and_packages() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "devDependencies": { "jest": "29" },
                    "jest": {
                        "testMatch": ["[", "*.test.js"],
                        "setupFiles": ["./missing.js", "./setup", "jest-plugin"]
                    }
                }"#,
            ),
            ("a.test.js", ""),
            ("setup.js", ""),
            ("index.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.test_files,
            BTreeSet::from(["a.test.js".into(), "setup.js".into()]),
            "an invalid glob or a missing file is left out"
        );
        assert_eq!(
            project.workspaces[0].test_packages,
            BTreeSet::from(["jest-plugin".into()])
        );
    }

    #[test]
    fn packages_of_the_repository_resolve_by_name_and_run_their_own_plugins() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "workspaces": ["apps/*", "packages/*"] }"#,
            ),
            (
                "apps/web/package.json",
                r#"{ "name": "web", "private": true, "dependencies": { "next": "15", "@x/ui": "*" } }"#,
            ),
            (
                "apps/web/app/page.tsx",
                r#"import { Button } from "@x/ui"; import { icon } from "@x/ui/icon"; export default () => [Button, icon];"#,
            ),
            (
                "packages/ui/package.json",
                r#"{ "name": "@x/ui", "private": true, "exports": { ".": { "types": "./x.d.ts", "import": "./src/index.mjs" } } }"#,
            ),
            ("packages/ui/src/index.mjs", "export const Button = 1;"),
            ("packages/ui/src/icon.mjs", "export const icon = 1;"),
            (
                "packages/pub/package.json",
                r#"{ "name": "pub", "exports": "./lib.js" }"#,
            ),
            ("packages/pub/lib.js", "exports.api = 1;"),
            (
                "packages/cond/package.json",
                r#"{ "name": "cond", "exports": { "require": "./main.cjs" } }"#,
            ),
            ("packages/cond/main.cjs", ""),
            (
                "packages/plain/package.json",
                r#"{ "name": "plain", "main": "lib/main.js" }"#,
            ),
            ("packages/plain/lib/main.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let dirs: Vec<&str> = project
            .workspaces
            .iter()
            .map(|workspace| workspace.dir.as_str())
            .collect();
        assert_eq!(
            dirs,
            [
                "",
                "apps/web",
                "packages/cond",
                "packages/plain",
                "packages/pub",
                "packages/ui"
            ]
        );
        assert_eq!(
            project.production_files,
            BTreeSet::from([
                "apps/web/app/page.tsx".into(),
                "packages/cond/main.cjs".into(),
                "packages/plain/lib/main.js".into(),
                "packages/pub/lib.js".into()
            ]),
            "the plugins of apps/web and the published packages' entries"
        );
        let page = &project.files["apps/web/app/page.tsx"];
        let targets: Vec<&str> = page.edges.iter().map(|edge| edge.target.as_str()).collect();
        assert_eq!(
            targets,
            ["packages/ui/src/index.mjs", "packages/ui/src/icon.mjs"]
        );
        assert_eq!(page.packages, BTreeSet::from(["@x/ui".to_string()]));
        assert_eq!(project.workspace_of("apps/web/app/page.tsx"), 1);
        assert_eq!(project.workspace_of("apps/website/x.js"), 0);
        assert_eq!(
            project.workspaces[1].production_packages,
            BTreeSet::from(["react".into(), "react-dom".into()])
        );
    }

    #[test]
    fn an_invalid_package_json_of_a_package_names_it() {
        let dir = project(&[
            ("package.json", r#"{ "workspaces": ["packages/*"] }"#),
            ("packages/a/package.json", "{ nope"),
        ]);
        let error = Project::load(dir.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("invalid package.json: packages/a/package.json: "),
            "{error}"
        );
    }

    #[test]
    fn without_package_json_nothing_is_declared() {
        let dir = project(&[("index.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        assert!(project.workspaces[0].declared_packages.is_empty());
    }

    #[test]
    fn an_invalid_package_json_is_an_error() {
        let dir = project(&[("package.json", "{ nope")]);
        let error = Project::load(dir.path()).unwrap_err();
        assert!(matches!(error, ProjectError::PackageJson(_)), "{error:?}");
        assert!(
            error.to_string().starts_with("invalid package.json: "),
            "{error}"
        );
    }

    #[test]
    fn a_file_a_plugin_compiles_is_a_source() {
        let dir = project(&[
            ("package.json", r#"{ "dependencies": { "vue": "3" } }"#),
            (
                "App.vue",
                "<template/>\n<script lang=\"ts\">import x from './x.js'; let y: number = x;</script>",
            ),
            ("Plain.vue", "<script>import z from './z.js'</script>"),
            ("x.js", "export default 1"),
            ("z.js", ""),
            ("C.svelte", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        let ids: Vec<&str> = project.files.keys().map(|id| id.as_str()).collect();
        assert_eq!(ids, ["App.vue", "Plain.vue", "x.js", "z.js"]);
        assert!(project.files["App.vue"].parse_error.is_none());
        assert!(
            project.files["Plain.vue"].facts.requires[0]
                .unused
                .is_empty()
        );
    }

    #[test]
    fn a_file_that_does_not_parse_is_in_the_graph_without_facts() {
        let dir = project(&[("lib/broken.js", "const = ;")]);
        let project = Project::load(dir.path()).unwrap();
        let broken = &project.files["lib/broken.js"];
        assert!(broken.parse_error.as_ref().is_some_and(|e| !e.is_empty()));
        assert!(broken.edges.is_empty());
    }

    #[test]
    fn a_package_json_is_read_as_far_as_its_fields_go() {
        let dir = project(&[
            (
                "package.json",
                "\u{feff}{ \"workspaces\": [\"packages/*\"], \"dependencies\": { \"a\": \"1\" } }",
            ),
            (
                "packages/b/package.json",
                r#"{
                    "name": 7,
                    "private": "true",
                    "main": ["no"],
                    "dependencies": ["not", "an", "object"],
                    "devDependencies": { "c": "1" },
                    "scripts": { "start": "node index.js", "odd": { "x": 1 } }
                }"#,
            ),
            ("packages/b/index.js", ""),
        ]);
        let loaded = Project::load(dir.path()).unwrap();
        let root = &loaded.workspaces[0];
        assert_eq!(root.dependencies.iter().collect::<Vec<_>>(), ["a"]);
        let b = &loaded.workspaces[1];
        assert!(b.dependencies.is_empty());
        assert_eq!(b.dev_dependencies.iter().collect::<Vec<_>>(), ["c"]);
        assert!(loaded.production_files.contains("packages/b/index.js"));
        let manifest = Manifest::load(dir.path(), "packages/b").unwrap();
        assert!(manifest.private && manifest.name.is_none() && manifest.main.is_none());
        let dir = project(&[("package.json", r#"{ "private": false }"#)]);
        assert!(!Manifest::load(dir.path(), "").unwrap().private);
    }

    #[test]
    fn an_entry_in_a_build_folder_is_also_looked_for_in_the_source() {
        let manifest = Manifest::from_value(&serde_json::json!({
            "exports": { ".": { "default": "./dist/source/index.js" } },
            "main": "build/main.js",
            "module": "./lib/esm/index.mjs"
        }));
        assert_eq!(
            manifest.entry_candidates(),
            [
                "./dist/source/index.js",
                "source/index.js",
                "src/source/index.js",
                "lib/source/index.js",
                "index.js",
                "src/index.js",
                "lib/index.js",
                "build/main.js",
                "main.js",
                "src/main.js",
                "lib/main.js",
                "./lib/esm/index.mjs",
                "esm/index.mjs",
                "src/esm/index.mjs",
                "lib/esm/index.mjs",
                "index.mjs",
                "src/index.mjs",
                "lib/index.mjs",
                "src/index",
                "index",
                "lib/index"
            ]
        );
    }

    #[test]
    fn a_library_whose_main_a_build_writes_is_found_by_its_name() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "name": "@scope/chai", "main": "./index.js" }"#,
            ),
            ("lib/chai.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.root_entries.iter().collect::<Vec<_>>(),
            ["lib/chai.js"]
        );
        let dir = project_private();
        std::fs::remove_file(dir.path().join("src/index.ts")).unwrap();
        std::fs::create_dir_all(dir.path().join("lib")).unwrap();
        std::fs::write(dir.path().join("lib/index.ts"), "").unwrap();
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.root_entries.iter().collect::<Vec<_>>(),
            ["lib/index.ts"]
        );
    }

    #[test]
    fn every_subpath_a_published_library_exports_is_an_entry() {
        let dir = project(&[
            (
                "package.json",
                r#"{
                    "name": "mailer",
                    "main": "./dist/cjs/mailer.js",
                    "exports": {
                        ".": { "import": "./dist/esm/mailer.js", "require": "./dist/cjs/mailer.js" },
                        "./lib/parser": { "node": { "require": "./dist/cjs/parser/index.js" } },
                        "./package.json": "./package.json",
                        "./types": { "types": "./dist/types.d.ts" },
                        "./internal/*": null,
                        "./fallback": ["./dist/cjs/fallback.js"]
                    }
                }"#,
            ),
            ("src/mailer.ts", ""),
            ("src/parser/index.ts", ""),
            ("src/internal.ts", ""),
            ("src/fallback.ts", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.root_entries.iter().collect::<Vec<_>>(),
            ["src/fallback.ts", "src/mailer.ts", "src/parser/index.ts"]
        );
        assert_eq!(
            project.production_files.iter().collect::<Vec<_>>(),
            ["src/fallback.ts", "src/mailer.ts", "src/parser/index.ts"]
        );
        let dir = project_private();
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.root_entries.iter().collect::<Vec<_>>(),
            ["src/index.ts"]
        );
        assert!(project.production_files.is_empty());
    }

    #[test]
    fn the_binaries_a_package_declares_are_entries() {
        let dir = project(&[
            (
                "package.json",
                r#"{ "private": true, "bin": { "tool": "./bin/tool.js", "other": "bin/other" } }"#,
            ),
            ("bin/tool.js", ""),
            ("bin/other.js", ""),
            (
                "packages/cli/package.json",
                r#"{ "name": "cli", "bin": "./dist/cli.js" }"#,
            ),
            ("packages/cli/src/cli.ts", ""),
        ]);
        std::fs::write(
            dir.path().join("package.json"),
            r#"{ "private": true, "workspaces": ["packages/*"], "bin": { "tool": "./bin/tool.js", "other": "bin/other" } }"#,
        )
        .unwrap();
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(
            project.production_files.iter().collect::<Vec<_>>(),
            ["bin/other.js", "bin/tool.js", "packages/cli/src/cli.ts"]
        );
    }

    #[test]
    fn a_tracked_file_an_ignore_pattern_matches_is_a_source_file() {
        let dir = project(&[
            (".gitignore", "mocha.js\n"),
            ("bin/mocha.js", ""),
            ("mocha.js", ""),
            ("lib/a.js", ""),
        ]);
        let git = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap();
            assert!(status.status.success(), "{status:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "-f", "bin/mocha.js", "lib/a.js"]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(ids(&project), ["bin/mocha.js", "lib/a.js"]);
    }

    fn project_private() -> tempfile::TempDir {
        project(&[
            (
                "package.json",
                r#"{ "private": true, "main": "dist/index.js" }"#,
            ),
            ("src/index.ts", ""),
        ])
    }

    fn set(ids: &[&str]) -> BTreeSet<FileId> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    fn entries(project: &Project, pattern: &str) -> Result<BTreeSet<FileId>, String> {
        project.entries(&[pattern.to_string()])
    }

    #[test]
    fn entries_are_paths_or_globs() {
        let dir = project(&[
            ("index.js", ""),
            ("lib/a.js", ""),
            ("lib/c.cjs", ""),
            ("lib/deep/b.js", ""),
        ]);
        let project = Project::load(dir.path()).unwrap();
        assert_eq!(entries(&project, "./index.js"), Ok(set(&["index.js"])));
        assert_eq!(entries(&project, "lib/*.js"), Ok(set(&["lib/a.js"])));
        assert_eq!(
            entries(&project, "lib/**"),
            Ok(set(&["lib/a.js", "lib/c.cjs", "lib/deep/b.js"]))
        );
        assert_eq!(
            project.entries(&["index.js".into(), "lib/*.cjs".into()]),
            Ok(set(&["index.js", "lib/c.cjs"]))
        );
    }

    #[test]
    fn an_entry_that_names_no_javascript_file_is_an_error() {
        let dir = project(&[("index.js", ""), ("data.json", "{}")]);
        let project = Project::load(dir.path()).unwrap();
        for pattern in ["nope.js", "data.json", "src/**/*.js"] {
            assert_eq!(
                entries(&project, pattern),
                Err(format!("entry matches no source file: {pattern}"))
            );
        }
    }

    #[test]
    fn an_invalid_glob_is_an_error() {
        let dir = project(&[("index.js", "")]);
        let project = Project::load(dir.path()).unwrap();
        let error = entries(&project, "lib/[a.js").unwrap_err();
        assert!(
            error.starts_with("invalid entry glob lib/[a.js: "),
            "{error}"
        );
    }
}
