# 2. Teste não mantém código vivo

## Objetivo

Código que só o próprio teste usa está morto em produção. O prunr separa o alcance de
**produção** do alcance de **teste** e trata isso como comportamento padrão, sem flag.

## Comportamento

- A partir de `entries.production` (ver [ponto 4](04-entry-points-config.md)) o prunr calcula o
  conjunto de arquivos e exports **vivos em produção**.
- A partir de `entries.test`, calcula o que é alcançado **só pelos testes**.
- Um arquivo ou export alcançado só por testes é reportado com o mesmo tipo de achado
  (`unused-file` / `unused-export`) e a marca `testOnly: true`, para que a mensagem diga
  "apague junto com o teste".
- Dependências: um pacote importado só por arquivos de teste e declarado em `dependencies` é
  reportado como `misplaced-dependency` (deveria estar em `devDependencies`).

## Critérios de aceite

- [ ] Fixture com um export usado só pelo `.test.js` → reportado com `testOnly: true`.
- [ ] Fixture com um arquivo importado só por teste → `unused-file` com `testOnly: true`.
- [ ] Fixture com um helper de teste (ex.: `__mocks__/`) que não é produção → não é reportado.
- [ ] Fixture com pacote usado só em teste e declarado em `dependencies` → `misplaced-dependency`.
- [ ] Um export usado em produção **e** em teste não é reportado.
