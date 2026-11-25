#!/usr/bin/env bash
# One-time setup of the npm packages, run by a maintainer before the first release.
#
# npm trusted publishing (OIDC) can only be configured on a package that already exists. This
# publishes a 0.0.0 placeholder of each package from your machine (npm asks for 2FA), then links
# each one to .github/workflows/release.yml, so every real release is published by CI without a
# token. Run it from a terminal where `npm whoami` shows the account that owns the @prunr org.
set -euo pipefail

REPO="ViniciusDev26/prunr"
PACKAGES=(prunr @prunr/linux-x64 @prunr/linux-arm64 @prunr/darwin-x64 @prunr/darwin-arm64 @prunr/win32-x64)

npm whoami > /dev/null

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
  npm deprecate "$name@0.0.0" "placeholder, install a real release"
done

for name in "${PACKAGES[@]}"; do
  npm trust github "$name" --file release.yml --repo "$REPO" --allow-publish --yes
done

echo "Done: the release workflow can now publish every package with OIDC."
