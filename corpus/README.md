# Corpus de validação

Projetos open source reais, fixados num commit, onde o prunr roda para achar falsos positivos,
erros e lentidão antes de quem usa. É o teste de aceite do [MVP](../docs/mvp/README.md) em
escala: cada divergência encontrada aqui vira uma fixture em `tests/fixtures/` antes da correção.

## Rodar

```sh
cargo build --release
node corpus/run.mjs                      # todos os projetos, só o prunr
node corpus/run.mjs --only express,koa   # alguns
node corpus/run.mjs --install --knip     # instala as dependências e compara com o Knip
node corpus/run.mjs --reinit             # gera de novo os prunr.json do `prunr init`
node corpus/run.mjs --jobs 1             # um projeto por vez: para os tempos do benchmark
```

Os projetos rodam em paralelo, um por núcleo (`--jobs` muda isso). Em paralelo, as ferramentas
disputam a CPU e os tempos medidos sobem: para os números do benchmark, use `--jobs 1`.

- Os checkouts e os resultados ficam fora do repositório, em `$PRUNR_CORPUS_DIR` (por padrão
  `~/.cache/prunr-corpus`): `repos/<nome>/` e `results/<nome>/{prunr,knip,diff}.json`, mais um
  `report.md` e um `results.json` com o resumo.
- Cada projeto é clonado com profundidade 1, no commit de [`repos.json`](repos.json).
- A configuração vem de `configs/<nome>.json` quando existe; senão, do `prunr init`, que fica
  salvo no checkout até um `--reinit`. Uma
  configuração do corpus só existe quando o `init` não basta, e diz o que um usuário teria de
  escrever.
- `--install` instala as dependências com o gerenciador do projeto, sem rodar scripts; o prunr
  usa `node_modules` para os binários dos scripts e os `peerDependencies`. `--knip` roda o Knip
  (`--knip-version`, padrão `5`) nos projetos com `node_modules` e compara arquivos, exports e
  dependências sem uso: o que os dois acham, só o prunr e só o Knip.

## Triagem

`node corpus/triage.mjs > triage.md` lista cada `missing-dependency` e `unresolved-require` do
último run, com a categoria e a evidência no disco de cada um (pacote irmão, declarado por outro
pacote, saída de build, caminho montado, submódulo, inexistente...).

Para cada achado que só o prunr tem, ou que parece errado: se é falso positivo, ele vira uma
fixture mínima que reproduz o padrão, depois a correção. Um achado real fica como está: o
corpus mede o prunr, não corrige os projetos.
