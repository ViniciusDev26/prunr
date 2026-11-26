# 23. Corpus de validação

## Objetivo

Achar falsos positivos, quebras e lentidão em projetos reais antes de quem usa, e medir o prunr
contra o Knip. É o teste de aceite do MVP em escala.

## Feito

- [`corpus/`](../../corpus/README.md): 100 projetos open source fixados num commit (CommonJS
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

  Os demais se sustentaram (58 na rodada final, com os 3 do Mocha que o `bin` passou a alcançar:
  `command` e `describe` de módulos de comando que o `cli.js` não lê mais): um `Symbol` local com o mesmo nome do export (undici), o mesmo nome
  vindo de outro módulo (pm2), propriedades do objeto exportado que ninguém lê (nodemon), e
  exports usados só por arquivos que nenhuma entrada alcança.

- Na triagem completa dos `unresolved-require` e `missing-dependency` do corpus:
  - um import `.js` resolve para o `.d.ts` ao lado (fastify);
  - especificadores com esquema (`data:`, `virtual:`) e o módulo nativo `bun` não são pacotes;
  - um pacote que o repositório versiona em `node_modules` (`test/node_modules/nr-test-utils`
    no node-red) não precisa ser declarado;
  - o que está em pastas de fixtures de testes não é reportado (as entradas de bundler do rollup e
    do webpack, as fixtures do codemod do TypeORM);
  - `require("..")` de uma pasta com `package.json` cujo `main` aponta para um build acha a entrada
    no fonte, também sob `lib/` (socket.io, ajv);
  - os `export` de dentro de um `namespace` (ou de um `declare module`) do TypeScript são membros
    dele, não exports do módulo (`Socket.DisconnectReason` no socket.io).

  - um `import("./x")` num exemplo de código de um comentário JSDoc não é import: só conta o de
    dentro de uma expressão de tipo (`@type {import("./x").Y}`) (webpack, tRPC);
  - o `moduleNameMapper` e os `__mocks__` do Jest e o `resolve.alias` do Vite resolvem os nomes
    deles (Jest: de 43 achados por alias para 16).

  Com isso, os `missing-dependency` do corpus caíram de 2.429 para 421.

- A triagem item a item: [`corpus/triage.mjs`](../../corpus/triage.mjs) confere cada
  `missing-dependency` e `unresolved-require` do último run no disco e escreve a categoria e a
  evidência de cada um. No run final, com os 100 projetos:

  | Achado | Categoria | Quantos | Evidência conferida |
  |---|---|---|---|
  | `missing-dependency` | pacote irmão sem declaração | 243 | o `package.json` do irmão tem esse `name` |
  | `missing-dependency` | declarado por outro pacote do repositório | 44 | o `package.json` que declara |
  | `missing-dependency` | módulo local que um alias resolve | 21 | o arquivo com esse nome (o Haste do próprio Jest, aliases dos casos de teste do webpack) |
  | `missing-dependency` | import de JSR do Deno | 5 | o mapa de imports do Deno |
  | `missing-dependency` | sem declaração | 151 | nenhum `package.json` do repositório o declara |
  | `unresolved-require` | caminho montado | 474 | o caminho só existe em tempo de execução |
  | `unresolved-require` | saída de build | 534 | a pasta para onde o build escreve |
  | `unresolved-require` | submódulo git não clonado | 23 | o `.gitmodules`; o corpus clona raso |
  | `unresolved-require` | sintaxe de loader | 1 | `loader!arquivo` do webpack |
  | `unresolved-require` | não existe | 185 | nada no caminho: imports quebrados de propósito nos testes de resolução do Jest, configs locais ignoradas pelo git, dados gerados por script, caminhos escritos para o teste compilado |

  Nenhuma categoria esconde um falso positivo do prunr: os que existiam viraram as correções
  acima.

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

- Na ampliação para 100 projetos (superagent, sinon, chai, postcss, less, bull, node-postgres,
  mysql2, o driver do MongoDB, node-redis, redux, dayjs, luxon, joi, yup, handlebars, marked,
  nock e outros): a entrada de uma biblioteca cujo `main` só o build escreve é achada pelo nome
  do pacote em `src/` ou `lib/` (`lib/chai.js`). Os 39 exports novos que só o prunr aponta foram
  conferidos um a um e se sustentam; 24 deles são constantes de protocolo do mysql2 em
  `module.exports = { CONNECT: 0x0b, ... }` que ninguém lê, e que o Knip não aponta porque trata
  o objeto como um export default único.

## Decisões e limitações

- **Um pacote publicado sem `exports`** deixa quem instala importar qualquer arquivo por caminho
  profundo (`ajv/dist/2020`). O prunr considera interface só as entradas que o `package.json`
  declara (`exports`, `main`, `module`, `bin`), como o Knip: tratar todo arquivo como público
  esconderia os exports mortos de qualquer biblioteca sem `exports`. Quem expõe caminhos
  profundos de propósito marca os arquivos com `@public` ou os lista em `entries.production`.
- **O Haste do Jest** (os módulos pelo nome nas fixtures internas do próprio Jest) e os aliases
  da configuração de cada caso de teste do webpack não são lidos: ferramentas desses próprios
  repositórios.

## Pendências

- Testes de tipo que o `tsc` checa pelo `include` do `tsconfig.json` (execa) e exemplos em
  Angular, Svelte e Vue (TanStack Query): dependem do plugin de TypeScript e dos compiladores da
  fase 2.
