# 23. Corpus de validação

## Objetivo

Achar falsos positivos, quebras e lentidão em projetos reais antes de quem usa, e medir o prunr
contra o Knip. É o teste de aceite do MVP em escala.

## Feito

- [`corpus/`](../../corpus/README.md): 30 projetos open source fixados num commit (CommonJS
  clássico, ESM e TypeScript, monorepos), um harness que roda o prunr e o Knip e compara arquivos,
  exports e dependências, e um [benchmark](../benchmark.md).
- Na primeira rodada, 11 projetos nem rodavam. A triagem virou fixtures e correções:
  - um arquivo com erro de sintaxe, JSX em `.js` e `return` no topo não param mais o check
    (`unparsed-file` quando o arquivo é alcançado);
  - `package.json` com BOM ou com campos de tipo inesperado (`"private": "true"`);
  - um `tsconfig.json` que estende um pacote não instalado não deixa tudo sem resolver;
  - pacotes aninhados que não são workspaces declaram para os arquivos abaixo deles; `bun:` é
    nativo;
  - a entrada declarada em `exports` é achada no fonte quando aponta para um build;
  - `NODE_OPTIONS` nos scripts, e scripts quebrados respeitando aspas;
  - um pacote que importa a si mesmo pelo nome, e a raiz de um monorepo importando os pacotes
    dele;
  - os test runners Mocha, `node --test`, tap, AVA, lab e borp;
  - projetos inline do Vitest e o `test.dir`.

## Pendências observadas

- O repositório do próprio Mocha não declara `mocha`, então o plugin não liga nele.
- Exports que só o Knip aponta (nodemailer, mongoose, webpack): conferir caso a caso quem está
  certo.
- Exemplos em Angular, Svelte e Vue (TanStack Query) dependem dos plugins e compiladores da fase 2.
- `unresolved-require` em volume no npm cli e no ESLint, e `missing-dependency` no Excalidraw:
  ainda sem triagem.
