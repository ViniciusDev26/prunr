//! The packages the `scripts` of `package.json` use: the binaries they run and the modules
//! they preload with `-r`/`--require`/`--import`/`--loader`.

use std::collections::{BTreeMap, BTreeSet};

/// The packages `script` uses. `bins` maps each binary to the package that declares it.
pub fn packages(script: &str, bins: &BTreeMap<String, String>) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for command in script.split([';', '|', '&', '\n']) {
        let words: Vec<&str> = command
            .split_whitespace()
            .map(|word| word.trim_matches(['"', '\'']))
            .collect();
        run(&words, bins, &mut used);
    }
    used
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
