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

## Adotando num código legado

Com `"baseline": ".prunr-baseline.json"` no `prunr.json`, `prunr baseline` grava os achados
atuais nesse arquivo (versione-o). A partir daí:

- os achados listados no baseline não falham o check;
- um achado **novo** falha;
- uma entrada do baseline que deixou de ser achada aparece como `stale-baseline` e também falha,
  até você rodar `prunr baseline` de novo. Assim a lista só diminui.

Cada entrada é identificada pelo tipo, arquivo e símbolo (ou pacote, ou ciclo), sem número de
linha: mover código não invalida o baseline. No JSON, as entradas obsoletas vêm em `"stale"`.
