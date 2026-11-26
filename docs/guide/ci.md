# CI e código legado

## No pull request

No GitHub Actions, `--github` transforma cada achado numa anotação no diff do PR, na linha do
export, do import, do `require` ou da dependência no `package.json`. `--since <ref>` mostra só os
achados que o commit `<ref>` não tinha: o prunr analisa também esse commit, num worktree
temporário do git, e compara. Um export que ficou sem uso porque o PR apagou o único consumidor
aparece mesmo num arquivo que o PR não tocou.

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0 # --since precisa do commit base
- run: npm ci
- run: npx prunr check --github --since origin/${{ github.base_ref }}
```

Rode a partir da raiz do repositório: os caminhos das anotações são relativos à raiz do projeto.

### A Action

A Action do prunr baixa a versão pedida do npm e roda o check, sem passo de instalação:

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0
- uses: ViniciusDev26/prunr@v0
  with:
    since: origin/${{ github.base_ref }}
```

| Entrada | Padrão | Para quê |
|---|---|---|
| `version` | `latest` | A versão do `@prunr/prunr`. |
| `working-directory` | `.` | A pasta do projeto, a do `prunr.json`. |
| `format` | `github` | `github` (anotações no diff), `sarif` (o code scanning do repositório; o job precisa de `security-events: write`) ou `markdown` (uma tabela no resumo do job). |
| `since` | | Só o que esse ref não tinha. |
| `production` | `false` | Só o que vai para produção. |

O passo falha quando há achados, e a saída `exit-code` diz o código do check.

## Outros formatos

| Flag | Para quê |
|---|---|
| `--json` | Máquinas: `{ "findings": [...], "stale": [...] }`. |
| `--github` | Anotações do GitHub Actions no diff do PR. |
| `--sarif` | SARIF 2.1.0, para o code scanning do GitHub (`github/codeql-action/upload-sarif`) e outras ferramentas de análise. Uma regra por tipo de achado. |
| `--codeclimate` | O relatório de Code Quality do GitLab (`artifacts: reports: codequality`), com um fingerprint estável por achado. |
| `--markdown` | Uma tabela para comentar no PR. |

Todos combinam com `--since` e com o baseline, e saem com o mesmo código de saída do check. As
entradas obsoletas do baseline aparecem como `stale-baseline`.

```yaml
# GitLab
prunr:
  script: npx prunr check --codeclimate > gl-code-quality-report.json
  artifacts:
    reports:
      codequality: gl-code-quality-report.json
```

## Só o que vai para produção

`prunr check --production` responde se o que vai para o deploy está limpo, sem o ruído do código
de desenvolvimento:

- arquivos e exports sem uso (inclusive os que só os testes usam) e dependências no lugar
  errado continuam;
- imports sem uso, dependências não declaradas, `require` não resolvido, arquivos que não fazem
  parse e comentários de ignorar sem uso só aparecem nos arquivos que a produção alcança;
  ciclos, só quando todos os arquivos do ciclo são de produção;
- `unused-dependency` só vale para `dependencies`: `devDependencies` não são checadas.

Combina com os formatos, com `--since` e com o baseline (as entradas do baseline que o modo
deixa de fora não viram `stale-baseline`).

## Adotando num código legado

Com `"baseline": ".prunr-baseline.json"` no `prunr.json`, `prunr baseline` grava os achados
atuais nesse arquivo (versione-o). A partir daí:

- os achados listados no baseline não falham o check;
- um achado **novo** falha;
- uma entrada do baseline que deixou de ser achada aparece como `stale-baseline` e também falha,
  até você rodar `prunr baseline` de novo. Assim a lista só diminui.

Cada entrada é identificada pelo tipo, arquivo e símbolo (ou pacote, ou ciclo), sem número de
linha: mover código não invalida o baseline. No JSON, as entradas obsoletas vêm em `"stale"`.
