# 6. CI de testes e release automático

## Objetivo

Todo PR é testado automaticamente, e publicar uma versão não exige nenhum passo manual além de
fazer merge do PR de release.

## CI (todo push e PR)

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test` em **Linux, macOS e Windows**
- Toolchain fixada (`rust-toolchain.toml` / `mise.toml`), a mesma localmente e no CI.

## Release automático

- **Versionamento:** commits no padrão Conventional Commits; o release-please mantém um PR de
  release com o próximo número de versão e o CHANGELOG.
- **Merge do PR de release** → tag → workflow de release:
  1. Compila os binários: Linux x64/arm64 (musl, estático), macOS x64/arm64, Windows x64.
  2. Anexa os binários e os `.sha256` ao GitHub Release.
  3. Publica no npm o pacote `@prunr/prunr` e um pacote por plataforma em `optionalDependencies`, sem
     `postinstall`.
  4. Publica com *trusted publishing* (OIDC) e *provenance*, sem token do npm guardado em secret.

### Decisões tomadas na implementação

- **Pacotes:** `@prunr/prunr` (o que se instala; o npm recusou `prunr` sem escopo por ser parecido demais com `lunr` e `grunt`) e `@prunr/linux-x64`, `@prunr/linux-arm64`,
  `@prunr/darwin-x64`, `@prunr/darwin-arm64`, `@prunr/win32-x64`, um por binário.
- **Só npm**; nada é publicado no crates.io (`publish = false`).
- **Trusted publishing (OIDC) desde o primeiro release, sem token npm.** Como o npm só configura
  OIDC num pacote que já existe, `scripts/bootstrap-npm.sh` publica um placeholder `0.0.0`
  (depreciado) de cada pacote e liga cada um ao workflow `release.yml`. Roda uma vez, na máquina
  de um mantenedor, com 2FA.
- **`RELEASE_PLEASE_TOKEN`:** um token próprio para o release-please, porque PRs abertos com o
  `GITHUB_TOKEN` não disparam workflows e o PR de release não passaria pelo ruleset.
- **A esteira inteira roda em todo PR:** testes, cobertura, os 5 builds, a checagem de binário
  estático e o `npm pack` de cada pacote. Todos são obrigatórios no ruleset da `main`.
- **Sem o `RELEASE_PLEASE_TOKEN`, o job do release-please só avisa e não abre o PR**, para a
  `main` não ficar vermelha enquanto o secret não existe.
- **A versão vem do `Cargo.toml`**, que o release-please atualiza; os `package.json` recebem a
  versão na hora de publicar.

## Critérios de aceite

- [x] Um PR com um teste quebrado fica vermelho no CI nos três sistemas (aconteceu no PR #3:
      o Windows expôs um bug de caminhos).
- [ ] A versão `0.1.0` é publicada de ponta a ponta pelo pipeline, sem nenhum comando manual.
- [ ] `npm i -D @prunr/prunr && npx prunr --version` funciona num runner Linux limpo, num macOS e num Windows.
- [x] O binário Linux roda numa imagem `alpine` e numa `debian` (prova de que é estático):
      verificado em todo PR, para x64 e arm64.
