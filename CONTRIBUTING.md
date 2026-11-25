# Contribuindo

O prunr é construído com **TDD e 100% de cobertura de testes**. As regras completas (TDD,
cobertura, plugins, commits) estão em [`CLAUDE.md`](CLAUDE.md), e o que vem depois no
[roadmap](docs/roadmap/).

## Ambiente

```sh
mise install                    # Rust, cargo-llvm-cov e committed nas versões do projeto
cargo test
cargo clippy --all-targets -- -D warnings
cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100
```

## Estrutura

- `src/`: `config` (prunr.json), `facts` (o que um arquivo exige, importa e exporta),
  `resolve` (para onde aponta um `require` ou `import`), `scripts` (o que os `scripts` do
  `package.json` rodam), `workspaces` (os pacotes de um monorepo), `plugins` (o que cada
  ferramenta, como Jest e Next.js, carrega por convenção ou configuração), `project` (o grafo),
  `checks`, `fix`, `report` e `cli`.
- `tests/fixtures/<caso>/`: um mini projeto por comportamento, rodado por `tests/check.rs`.
- `npm/`: o pacote `@prunr/prunr`, o launcher que executa o binário da plataforma e o script que
  publica tudo.
- `assets/brand/`: o logo, gerado por `generate.py`.

## Pull requests

Commits e títulos de PR em inglês, seguindo Conventional Commits. Todo PR passa pela esteira
inteira: testes nos três sistemas, cobertura de 100%, os 5 builds, a checagem de binário estático
e o `npm pack` de cada pacote. A `main` só aceita PRs com tudo verde, por squash merge.

## Releases

O [release-please](https://github.com/googleapis/release-please) mantém um PR de release aberto
com a próxima versão e o CHANGELOG. Ao fazer merge dele, o workflow `release`:

1. compila os 5 binários e confere que os de Linux rodam em Alpine e Debian 11;
2. anexa os binários e os `.sha256` ao release do GitHub;
3. publica `@prunr/<plataforma>` e `@prunr/prunr` no npm com *trusted publishing* (OIDC, sem
   token);
4. instala `@prunr/prunr` do npm em Linux, macOS e Windows e roda `npx prunr --version`.

### Um pacote npm novo

O npm só aceita configurar *trusted publishing* num pacote que já existe. Por isso, um pacote
novo (por exemplo, o de uma plataforma nova) é criado uma vez, à mão, por um dono da org
`@prunr`, antes do release que o publica:

1. Publicar um placeholder `0.0.0`: numa pasta vazia, um `package.json` com `name`,
   `"version": "0.0.0"`, `license` e `repository`, e `npm publish --access public`.
2. Esperar o npm processar o pacote novo (`npm view <pacote>@0.0.0` responde), o que leva alguns
   minutos.
3. Opcional: `npm deprecate <pacote>@0.0.0 "placeholder, install a real release"`.
4. Ligar o pacote ao workflow de release:
   `npm trust github <pacote> --file release.yml --repo ViniciusDev26/prunr --allow-publish --yes`.
