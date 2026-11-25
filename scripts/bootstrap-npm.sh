#!/usr/bin/env bash
# One-time setup of the npm packages, run by a maintainer before the first release.
#
# npm trusted publishing (OIDC) can only be configured on a package that already exists. This
# publishes a 0.0.0 placeholder of each package from your machine (npm asks for 2FA), then links
# each one to .github/workflows/release.yml, so every real release is published by CI without a
# token. Run it from a terminal where `npm whoami` shows an owner of the @prunr org.
#
# It can run again: packages that exist are skipped, and every step waits for npm to finish
# processing a new package before touching it.
set -euo pipefail

REPO="ViniciusDev26/prunr"
PACKAGES=(@prunr/prunr @prunr/linux-x64 @prunr/linux-arm64 @prunr/darwin-x64 @prunr/darwin-arm64 @prunr/win32-x64)

npm whoami > /dev/null

published() {
  npm view "$1@0.0.0" version > /dev/null 2>&1
}

echo "== publish the placeholders"
for name in "${PACKAGES[@]}"; do
  if npm view "$name" version > /dev/null 2>&1; then
    echo "$name already exists"
    continue
  fi
  dir=$(mktemp -d)
  cat > "$dir/package.json" <<JSON
{
  "name": "$name",
  "version": "0.0.0",
  "description": "Placeholder: the real package is published by the prunr release workflow.",
  "license": "MIT",
  "repository": { "type": "git", "url": "git+https://github.com/$REPO.git" }
}
JSON
  (cd "$dir" && npm publish --access public)
done

echo "== wait for npm to process them"
for name in "${PACKAGES[@]}"; do
  for _ in $(seq 1 30); do
    published "$name" && break
    sleep 10
  done
  published "$name" || { echo "$name@0.0.0 is still not available, run this again later"; exit 1; }
  echo "$name@0.0.0 is available"
done

echo "== deprecate the placeholders"
for name in "${PACKAGES[@]}"; do
  npm deprecate "$name@0.0.0" "placeholder, install a real release" ||
    echo "could not deprecate $name@0.0.0 (cosmetic, safe to ignore)"
done

echo "== link each package to the release workflow"
for name in "${PACKAGES[@]}"; do
  npm trust github "$name" --file release.yml --repo "$REPO" --allow-publish --yes
done

echo "Done: the release workflow can now publish every package with OIDC."
