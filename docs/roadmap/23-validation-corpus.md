# 23. Corpus de validação

## Objetivo

Achar falsos positivos, quebras e lentidão em projetos reais antes de quem usa, e medir o prunr
contra o Knip. É o teste de aceite do MVP em escala.

## Feito

- [`corpus/`](../../corpus/README.md): 55 projetos open source fixados num commit (CommonJS
  clássico, middlewares do Express, ESM e TypeScript, monorepos), um harness que roda o prunr e o
  Knip em paralelo e compara arquivos, exports e dependências, e um [benchmark](../benchmark.md).
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

- Na ampliação para 55 projetos:
  - um arquivo que passa o próprio `exports` para uma função usa todos os exports (debug);
  - `.mocharc.json` com comentários, spec numa pasta de build, a pasta de teste de um runner
    inteira como teste, o tsd, imports que resolvem para `.d.ts` e o plugin ligado no repositório
    da própria ferramenta;
  - cada subcaminho do `exports` de um pacote publicado é entrada, procurada no fonte (nodemailer).

- Na conferência, um a um, dos 70 exports que só o prunr apontava:
  - o `bin` de um pacote é entrada de produção (Mocha, knex);
  - passar o valor exportado para uma função (`var God = module.exports = {...}`, depois
    `fn(God)`) usa todos os exports (pm2);
  - um arquivo que o git rastreia é fonte mesmo quando um padrão do `.gitignore` casa com ele
    (`mocha.js`, para um bundle na raiz, também casava com `bin/mocha.js`).

- Os scripts que repassam argumentos a outro (`npm run unit -- --config x`) e a ferramenta chamada
  pelo próprio `bin` no repositório dela (`node bin/mocha.js`): no repositório do Mocha, os
  arquivos sem uso caíram de 425 para 19.

  Os demais se sustentaram: um `Symbol` local com o mesmo nome do export (undici), o mesmo nome
  vindo de outro módulo (pm2), propriedades do objeto exportado que ninguém lê (nodemon), e
  exports usados só por arquivos que nenhuma entrada alcança.

## O que a triagem decidiu manter

Diferenças conferidas em que o achado do prunr se sustenta, ou que é do Knip:

- **`unresolved-require` em `generated/` e `dist/`** (jsdom, ajv): arquivos que só existem
  depois do build; o aviso é legítimo.
- **`require` com caminho montado** (`require(path + '/package-lock.json')` no npm cli, no
  `Makefile.js` do ESLint): avisos legítimos.
- **Exemplos que importam pacotes irmãos sem declarar** (Excalidraw): o exemplo não instala
  sozinho; é achado real pela regra dos monorepos.
- **O pacote raiz sem `name`** (helmet), cujos harnesses de teste instalam o pacote empacotado:
  nada o declara.
- **Fixtures de entrada do codemod do TypeORM** que importam `typeorm`, e ciclos entre entidades
  de teste: verdadeiros pela regra; o `--production` os deixa de fora.
- **Exports que só o Knip aponta:** no nodemailer, tipos de subcaminhos publicados (interface
  pública, depois da correção das entradas); no mongoose, exports lidos no próprio arquivo
  (falso positivo do Knip); no webpack, os tipos de `declarations/WebpackOptions.ts` que outro
  tipo do mesmo arquivo usa (`UmdAmdContainer = DottedIdentifier`). Esse último é uma diferença
  de semântica: o prunr não reporta um export que o próprio arquivo usa, e o Knip reporta, a menos
  que se ligue o `ignoreExportsUsedInFile` dele.

## Pendências

- Testes de tipo que o `tsc` checa pelo `include` do `tsconfig.json` (execa) e exemplos em
  Angular, Svelte e Vue (TanStack Query): dependem do plugin de TypeScript e dos compiladores da
  fase 2.
