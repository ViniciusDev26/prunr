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
  3. Publica no npm o pacote `prunr` e um pacote por plataforma em `optionalDependencies`, sem
     `postinstall`.
  4. Publica com *trusted publishing* (OIDC) e *provenance*, sem token do npm guardado em secret.

## Critérios de aceite

- [ ] Um PR com um teste quebrado fica vermelho no CI nos três sistemas.
- [ ] A versão `0.1.0` é publicada de ponta a ponta pelo pipeline, sem nenhum comando manual.
- [ ] `npm i -D prunr && npx prunr --version` funciona num runner Linux limpo, num macOS e num Windows.
- [ ] O binário Linux roda numa imagem `alpine` e numa `debian` (prova de que é estático).
