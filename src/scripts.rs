//! The packages the `scripts` of `package.json` use: the binaries they run and the modules
//! they preload with `-r`/`--require`/`--import`/`--loader`.

use std::collections::{BTreeMap, BTreeSet};

/// The packages `script` uses. `bins` maps each binary to the package that declares it.
pub fn packages(script: &str, bins: &BTreeMap<String, String>) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for words in commands(script) {
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        // `NODE_OPTIONS='--import=tsx/esm'` preloads for every node the command starts.
        for word in &words {
            if let Some(options) = word.strip_prefix("NODE_OPTIONS=") {
                preloads(&options.split_whitespace().collect::<Vec<_>>(), &mut used);
            }
        }
        run(&words, bins, &mut used);
        for word in &words {
            if let Some((_, inside)) = word.split_once("node_modules/")
                && !inside.is_empty()
            {
                used.insert(crate::resolve::package_name(inside).to_string());
            }
        }
    }
    used
}

/// The configuration files a script passes to `runner` with `--config`/`-c`.
pub fn configs(script: &str, runner: &str) -> Vec<String> {
    let installed = format!("node_modules/{runner}/");
    let mut found = Vec::new();
    for words in commands(script) {
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        let runs = words
            .iter()
            .any(|word| *word == runner || word.contains(&installed));
        if !runs {
            continue;
        }
        for (index, word) in words.iter().enumerate() {
            let value = match word.split_once('=') {
                Some(("--config" | "-c", value)) => Some(value),
                _ if ["--config", "-c"].contains(word) => words.get(index + 1).copied(),
                _ => None,
            };
            found.extend(value.map(str::to_string));
        }
    }
    found
}

/// A file a command of a script runs with Node.js or a runner like it (`node`, `tsx`,
/// `ts-node`, `bun`...).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Run {
    /// The package of the repository it runs in (`pnpm --filter <name>`, `npm -w <name>`,
    /// `yarn workspace <name>`), when not the script's own.
    pub package: Option<String>,
    /// The files it preloads (`-r ./setup.js`, `--import ./register.js`).
    pub preloads: Vec<String>,
    /// The arguments that can name the file, in order: the first that is a source file is it.
    pub candidates: Vec<String>,
}

/// The files the commands of `script` run.
pub fn files(script: &str) -> Vec<Run> {
    commands(script)
        .iter()
        .filter_map(|words| file_run(&words.iter().map(String::as_str).collect::<Vec<_>>()))
        .collect()
}

/// Runners whose first argument that is a file is the file they run.
const FILE_RUNNERS: &[&str] = &[
    "node",
    "tsx",
    "ts-node",
    "ts-node-esm",
    "ts-node-dev",
    "tsnd",
    "babel-node",
    "nodemon",
    "vite-node",
    "jiti",
    "esno",
    "bun",
];
/// Flags of those runners that take the next word as their value.
const VALUE_FLAGS: &[&str] = &[
    "--env-file",
    "--env-file-if-exists",
    "-C",
    "--conditions",
    "--tsconfig",
    "--watch-path",
    "--input-type",
    "--title",
];
/// nodemon's flags that take a value; its `-e` is the extensions, not code to run.
const NODEMON_VALUE_FLAGS: &[&str] = &[
    "-e", "--ext", "-w", "--watch", "-x", "--exec", "-i", "--ignore", "-d", "--delay", "--signal",
];
/// Flags after which there is no file: the code comes in the command line.
const EVAL: &[&str] = &["-e", "--eval", "-p", "--print"];
/// Wrappers that run the rest of the command: `cross-env A=1 node x`, `dotenv -- node x`.
const WRAPPERS: &[&str] = &["cross-env", "cross-env-shell", "env"];

/// The file one command runs, through package managers, runners and wrappers.
fn file_run(words: &[&str]) -> Option<Run> {
    let mut words = words;
    let mut package = None;
    loop {
        words = skip(words, is_assignment);
        let (&first, rest) = words.split_first()?;
        if RUNNERS.contains(&first) || WRAPPERS.contains(&first) {
            words = skip(rest, |word| word.starts_with('-'));
            continue;
        }
        if first == "dotenv" {
            let end = rest.iter().position(|word| *word == "--")?;
            words = &rest[end + 1..];
            continue;
        }
        if MANAGERS.contains(&first) && !(first == "bun" && rest.first() != Some(&"x")) {
            let rest = manager_options(first, rest, &mut package);
            match rest.split_first() {
                Some((&"workspace", [name, rest @ ..])) if first == "yarn" => {
                    package = Some(name.to_string());
                    words = rest;
                }
                Some((sub, rest)) if EXEC.contains(sub) => {
                    words = manager_options(first, rest, &mut package);
                }
                Some(_) if first != "npm" => words = rest,
                _ => return None,
            }
            continue;
        }
        if !FILE_RUNNERS.contains(&first) {
            return None;
        }
        let mut run = Run {
            package,
            ..Run::default()
        };
        let mut index = 0;
        while let Some(&word) = rest.get(index) {
            index += 1;
            if first != "nodemon" && EVAL.contains(&word) {
                break;
            }
            if let Some((flag, value)) = word.split_once('=') {
                if PRELOAD.contains(&flag) && value.starts_with(['.', '/']) {
                    run.preloads.push(value.to_string());
                }
                continue;
            }
            let takes_value = PRELOAD.contains(&word)
                || VALUE_FLAGS.contains(&word)
                || (first == "nodemon" && NODEMON_VALUE_FLAGS.contains(&word));
            if takes_value {
                if let Some(value) = rest.get(index)
                    && PRELOAD.contains(&word)
                    && value.starts_with(['.', '/'])
                {
                    run.preloads.push(value.to_string());
                }
                index += 1;
            } else if !word.starts_with('-') {
                run.candidates.push(word.to_string());
            }
        }
        return Some(run);
    }
}

/// `words` after a package manager's leading options, noting the package one of them names
/// (`pnpm --filter <name>`, `npm --workspace <name>`).
fn manager_options<'w, 's>(
    manager: &str,
    words: &'w [&'s str],
    package: &mut Option<String>,
) -> &'w [&'s str] {
    let names: &[&str] = match manager {
        "pnpm" => &["--filter", "-F"],
        "npm" => &["--workspace", "-w"],
        _ => &[],
    };
    let mut words = words;
    while let Some((&word, rest)) = words.split_first() {
        if let Some((flag, value)) = word.split_once('=')
            && names.contains(&flag)
        {
            *package = Some(value.to_string());
            words = rest;
        } else if names.contains(&word) {
            *package = rest.first().map(|name| name.to_string());
            words = rest.get(1..).unwrap_or_default();
        } else if word.starts_with('-') {
            words = rest;
        } else {
            break;
        }
    }
    words
}

/// The words of each command of a script.
/// The words of each command of a script, split as a shell would: by whitespace and by `;`,
/// `|`, `&` and newlines, except inside quotes, which are dropped
/// (`NODE_OPTIONS='--import tsx' node a.js` → `NODE_OPTIONS=--import tsx`, `node`, `a.js`).
fn commands(script: &str) -> Vec<Vec<String>> {
    let mut commands = vec![Vec::new()];
    let mut word: Option<String> = None;
    let mut quote = None;
    for c in script.chars() {
        match (quote, c) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), c) => word.get_or_insert_with(String::new).push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                word.get_or_insert_with(String::new);
            }
            (None, c) if c.is_whitespace() || matches!(c, ';' | '|' | '&') => {
                let command = commands.last_mut().expect("there is always a command");
                command.extend(word.take());
                if !c.is_whitespace() || c == '\n' {
                    commands.push(Vec::new());
                }
            }
            (None, c) => word.get_or_insert_with(String::new).push(c),
        }
    }
    commands
        .last_mut()
        .expect("there is always a command")
        .extend(word);
    commands
}

/// Package managers that run a binary: as `yarn jest`, or through a subcommand (`pnpm exec jest`).
const MANAGERS: &[&str] = &["npm", "yarn", "pnpm", "bun"];
/// Their subcommands that run a binary.
const EXEC: &[&str] = &["exec", "dlx", "x"];
/// Package runners: `npx jest`.
const RUNNERS: &[&str] = &["npx", "bunx", "pnpx"];
/// Flags that preload a module, in any command (`node -r`, `mocha --require`).
const PRELOAD: &[&str] = &[
    "-r",
    "--require",
    "--import",
    "--loader",
    "--experimental-loader",
];

/// The packages one command uses.
fn run(words: &[&str], bins: &BTreeMap<String, String>, used: &mut BTreeSet<String>) {
    let mut words = words;
    loop {
        words = skip(words, is_assignment);
        let Some((&first, rest)) = words.split_first() else {
            return;
        };
        if RUNNERS.contains(&first) {
            words = skip(rest, |word| word.starts_with('-'));
            continue;
        }
        if MANAGERS.contains(&first) {
            match rest.first() {
                Some(sub) if EXEC.contains(sub) => {
                    words = skip(&rest[1..], |word| word.starts_with('-'));
                    continue;
                }
                Some(_) if first != "npm" => {
                    words = rest;
                    continue;
                }
                _ => return,
            }
        }
        preloads(rest, used);
        let Some(package) = bins.get(first) else {
            return;
        };
        used.insert(package.clone());
        match first {
            "cross-env" => words = rest,
            "dotenv" => match rest.iter().position(|word| *word == "--") {
                Some(end) => words = &rest[end + 1..],
                None => return,
            },
            _ => return,
        }
    }
}

/// `words` without the leading ones that `skipped` accepts.
fn skip<'w, 's>(words: &'w [&'s str], skipped: impl Fn(&str) -> bool) -> &'w [&'s str] {
    let start = words
        .iter()
        .position(|word| !skipped(word))
        .unwrap_or(words.len());
    &words[start..]
}

/// `NAME=value`.
fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty() && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
    })
}

/// The packages preloaded with `-r x`, `--require=x` and the like.
fn preloads(words: &[&str], used: &mut BTreeSet<String>) {
    for (index, word) in words.iter().enumerate() {
        let value = match word.split_once('=') {
            Some((flag, value)) if PRELOAD.contains(&flag) => Some(value),
            _ if PRELOAD.contains(word) => words.get(index + 1).copied(),
            _ => None,
        };
        if let Some(value) = value
            && !value.starts_with(['.', '/'])
        {
            used.insert(crate::resolve::package_name(value).to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(package: Option<&str>, preloads: &[&str], candidates: &[&str]) -> Run {
        Run {
            package: package.map(str::to_string),
            preloads: preloads.iter().map(|p| p.to_string()).collect(),
            candidates: candidates.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn runners_name_the_file_they_run_after_their_flags() {
        assert_eq!(
            files(
                "NODE_ENV=production node --env-file .env -r ./a.js --import=./b.js -r dotenv/config --inspect=0.0.0.0 src/server.js arg"
            ),
            [run(None, &["./a.js", "./b.js"], &["src/server.js", "arg"])]
        );
        assert_eq!(
            files("tsx watch ./src/index.ts"),
            [run(None, &[], &["watch", "./src/index.ts"])]
        );
        assert_eq!(
            files("bun run src/job.ts"),
            [run(None, &[], &["run", "src/job.ts"])]
        );
        assert_eq!(
            files("nodemon -e ts,json --watch src -x tsx src/index.ts"),
            [run(None, &[], &["src/index.ts"])]
        );
        assert_eq!(files("node -r"), [run(None, &[], &[])]);
    }

    #[test]
    fn code_in_the_command_line_runs_no_file() {
        assert_eq!(
            files("node -e \"require('./x.js')\""),
            [run(None, &[], &[])]
        );
        assert_eq!(files("node --print 1 x.js"), [run(None, &[], &[])]);
    }

    #[test]
    fn wrappers_runners_and_managers_lead_to_the_file() {
        for script in [
            "cross-env A=1 tsx x.ts",
            "env A=1 tsx x.ts",
            "dotenv -e .env -- tsx x.ts",
            "npx --yes tsx x.ts",
            "pnpm exec tsx x.ts",
            "yarn tsx x.ts",
            "bun x tsx x.ts",
            "npm exec -- tsx x.ts",
        ] {
            assert_eq!(files(script), [run(None, &[], &["x.ts"])], "{script}");
        }
        assert!(files("dotenv -e .env tsx x.ts").is_empty());
        assert!(files("npm run build && jest && pnpm").is_empty());
    }

    #[test]
    fn a_manager_can_run_the_file_in_another_package() {
        for script in [
            "pnpm --filter @app/tools exec tsx x.ts",
            "pnpm -F=@app/tools exec tsx x.ts",
            "pnpm --silent --filter @app/tools exec tsx x.ts",
            "npm exec -w @app/tools -- tsx x.ts",
            "npm --workspace=@app/tools exec tsx x.ts",
            "yarn workspace @app/tools tsx x.ts",
        ] {
            assert_eq!(
                files(script),
                [run(Some("@app/tools"), &[], &["x.ts"])],
                "{script}"
            );
        }
        assert_eq!(files("pnpm --filter"), []);
    }

    fn bins() -> BTreeMap<String, String> {
        [
            ("jest", "jest"),
            ("tsc", "typescript"),
            ("cross-env", "cross-env"),
            ("dotenv", "dotenv-cli"),
            ("nodemon", "nodemon"),
            ("eslint", "eslint"),
        ]
        .into_iter()
        .map(|(bin, package)| (bin.to_string(), package.to_string()))
        .collect()
    }

    fn used(script: &str) -> Vec<String> {
        packages(script, &bins()).into_iter().collect()
    }

    #[test]
    fn node_options_preload_packages_too() {
        assert_eq!(
            used("NODE_OPTIONS='--import=tsx/esm --require dotenv/config' node --test 'test/*.ts'"),
            ["dotenv", "tsx"]
        );
        assert_eq!(used("NODE_OPTIONS=--import=tsx jest"), ["jest", "tsx"]);
    }

    #[test]
    fn a_binary_names_its_package() {
        assert_eq!(used("jest --coverage"), ["jest"]);
        assert_eq!(used("tsc -p ."), ["typescript"]);
    }

    #[test]
    fn a_binary_no_package_declares_is_nothing() {
        assert!(used("docker compose up && git status").is_empty());
    }

    #[test]
    fn every_command_of_a_script_counts() {
        assert_eq!(
            used("jest && eslint . || tsc; nodemon a.js | cat & echo\nrm -rf x 2>&1"),
            ["eslint", "jest", "nodemon", "typescript"]
        );
    }

    #[test]
    fn environment_assignments_are_skipped() {
        assert_eq!(used("NODE_ENV=test TZ=UTC jest"), ["jest"]);
    }

    #[test]
    fn quotes_around_a_word_are_dropped() {
        assert_eq!(used(r#""jest" 'a b'"#), ["jest"]);
    }

    #[test]
    fn package_runners_run_the_binary_after_their_flags() {
        for script in [
            "npx jest",
            "npx --yes jest",
            "bunx jest",
            "pnpx jest",
            "pnpm exec jest",
            "pnpm dlx jest",
            "yarn exec jest",
            "npm exec -- jest",
            "npm x jest",
            "yarn jest",
            "pnpm jest",
            "bun x jest",
        ] {
            assert_eq!(used(script), ["jest"], "{script}");
        }
    }

    #[test]
    fn running_another_script_is_not_a_binary() {
        for script in [
            "npm run jest",
            "yarn run jest",
            "pnpm run jest",
            "npm test",
            "bun run jest",
        ] {
            assert!(used(script).is_empty(), "{script}");
        }
    }

    #[test]
    fn wrappers_run_the_command_after_them() {
        assert_eq!(
            used("cross-env NODE_ENV=production tsc"),
            ["cross-env", "typescript"]
        );
        assert_eq!(used("dotenv -e .env -- jest"), ["dotenv-cli", "jest"]);
        assert_eq!(used("dotenv -e .env"), ["dotenv-cli"]);
    }

    #[test]
    fn a_file_inside_node_modules_uses_its_package() {
        assert_eq!(
            used(
                "node --flag node_modules/jest/bin/jest.js && node ./node_modules/@scope/tool/cli.js"
            ),
            ["@scope/tool", "jest"]
        );
        assert!(used("node node_modules/").is_empty());
    }

    #[test]
    fn the_config_a_script_passes_to_a_runner_is_found() {
        let found = |script: &str| configs(script, "jest");
        assert_eq!(
            found("jest --config jest.e2e.config.js && jest -c=./other.json; jest --config=x.js"),
            ["jest.e2e.config.js", "./other.json", "x.js"]
        );
        assert_eq!(
            found("node node_modules/jest/bin/jest.js --config jest.integration.config.js"),
            ["jest.integration.config.js"]
        );
        assert!(found("vitest --config vitest.e2e.ts").is_empty());
        assert!(found("jest --config").is_empty());
        assert_eq!(
            configs("npx vitest run -c vitest.e2e.ts", "vitest"),
            ["vitest.e2e.ts"]
        );
    }

    #[test]
    fn preloaded_modules_are_used() {
        assert_eq!(
            used(
                "node -r dotenv/config --require=ts-node/register --import @swc-node/register/esm \
                 --loader=tsx --experimental-loader esm-loader -r ./local.js --require /abs.js"
            ),
            [
                "@swc-node/register",
                "dotenv",
                "esm-loader",
                "ts-node",
                "tsx"
            ]
        );
        assert_eq!(used("jest -r"), ["jest"]);
    }
}
