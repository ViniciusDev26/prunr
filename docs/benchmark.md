# Benchmark

O prunr e o Knip nos 100 projetos do [corpus de validação](../corpus/README.md), no mesmo commit
de cada projeto, com as dependências instaladas. Os números são de uma máquina Linux (x86-64,
32 núcleos), com o prunr de release e o Knip 5 rodado por `npx`, um
projeto por vez (`--jobs 1`). O tempo do Knip inclui a partida do Node.

Para reproduzir: `cargo build --release && node corpus/run.mjs --install --knip --jobs 1`.

## Tempo

Os 85 projetos em que os dois rodaram, do mais lento para o Knip:

| Projeto | prunr | Knip |
|---|---|---|
| webpack | 1,14 s | 6,86 s |
| date-fns | 0,10 s | 4,75 s |
| mongoose | 0,03 s | 2,93 s |
| cli | 0,09 s | 2,88 s |
| fastify | 0,02 s | 2,74 s |
| core | 0,02 s | 2,54 s |
| rollup | 0,33 s | 2,51 s |
| bullmq | 0,03 s | 2,47 s |
| agenda | 0,02 s | 2,42 s |
| node-postgres | 0,02 s | 2,29 s |
| undici | 0,03 s | 2,22 s |
| node-redis | 0,04 s | 2,19 s |
| pino | 0,01 s | 2,18 s |
| mocha | 0,02 s | 2,18 s |
| less | 0,02 s | 2,13 s |
| got | 0,02 s | 2,13 s |
| handlebars | 0,01 s | 2,11 s |
| axios | 0,02 s | 2,00 s |
| sinon | 0,02 s | 2,00 s |
| commander | 0,02 s | 1,97 s |
| socket.io | 0,05 s | 1,96 s |
| cheerio | 0,01 s | 1,96 s |
| nodemailer | 0,02 s | 1,96 s |
| jsdom | 0,03 s | 1,94 s |
| node-mongodb-native | 0,19 s | 1,94 s |
| execa | 0,02 s | 1,92 s |
| knex | 0,03 s | 1,91 s |
| node-cron | 0,01 s | 1,86 s |
| http-proxy-middleware | 0,01 s | 1,86 s |
| nock | 0,01 s | 1,78 s |
| postcss | 0,01 s | 1,78 s |
| bull | 0,01 s | 1,77 s |
| ramda | 0,02 s | 1,77 s |
| dayjs | 0,01 s | 1,77 s |
| express | 0,01 s | 1,75 s |
| pm2 | 0,03 s | 1,73 s |
| marked | 0,01 s | 1,67 s |
| body-parser | 0,01 s | 1,64 s |
| ioredis | 0,03 s | 1,64 s |
| chai | 0,01 s | 1,63 s |
| lodash | 0,02 s | 1,62 s |
| yargs | 0,01 s | 1,62 s |
| superagent | 0,01 s | 1,62 s |
| node-restify | 0,01 s | 1,61 s |
| luxon | 0,01 s | 1,61 s |
| helmet | 0,01 s | 1,61 s |
| redux | 0,01 s | 1,59 s |
| node-mysql2 | 0,02 s | 1,57 s |
| ws | 0,01 s | 1,54 s |
| nunjucks | 0,02 s | 1,53 s |
| moment | 0,02 s | 1,53 s |
| node-fetch | 0,01 s | 1,52 s |
| winston | 0,01 s | 1,52 s |
| joi | 0,01 s | 1,52 s |
| node-semver | 0,01 s | 1,52 s |
| hapi | 0,01 s | 1,51 s |
| multer | 0,01 s | 1,49 s |
| ajv | 0,01 s | 1,47 s |
| node-red | 0,02 s | 1,46 s |
| nodemon | 0,01 s | 1,46 s |
| chalk | 0,01 s | 1,46 s |
| validator | 0,01 s | 1,45 s |
| koa | 0,01 s | 1,43 s |
| uuid | 0,01 s | 1,42 s |
| node-jsonwebtoken | 0,01 s | 1,41 s |
| supertest | 0,01 s | 1,40 s |
| nyc | 0,01 s | 1,39 s |
| debug | 0,01 s | 1,39 s |
| path-to-regexp | 0,01 s | 1,38 s |
| session | 0,01 s | 1,36 s |
| mysql | 0,01 s | 1,36 s |
| gulp | 0,01 s | 1,36 s |
| morgan | 0,01 s | 1,36 s |
| node-schedule | 0,01 s | 1,35 s |
| ejs | 0,01 s | 1,35 s |
| node-bunyan | 0,01 s | 1,34 s |
| node-http-proxy | 0,01 s | 1,33 s |
| json-server | 0,01 s | 1,32 s |
| dotenv | 0,01 s | 1,32 s |
| node-glob | 0,01 s | 1,32 s |
| log4js-node | 0,01 s | 1,31 s |
| cors | 0,01 s | 1,31 s |
| passport | 0,01 s | 1,29 s |
| nanoid | 0,01 s | 1,29 s |
| grunt | 0,01 s | 1,23 s |

O webpack tem mais de 24 mil arquivos-fonte. Num monorepo privado de 13 mil arquivos, fora do
corpus, o check leva 0,6 s.

## Onde só um dos dois roda

O prunr rodou nos 100. O Knip carrega as configurações das ferramentas executando-as, e em 7
projetos (ESLint, Vite, tRPC, TanStack Query, Excalidraw, browserify, markdown-it) parou num erro
ao carregar uma delas. Em outros 8 (Prettier, Nest, Jest, Sequelize, TypeORM, Zod, AVA, Yup) a
instalação das dependências falhou (conflito de peers, versões antigas do npm), e o Knip não roda
sem elas.

## Concordância

Nos 85 projetos em que os dois rodaram, somando todos:

| | Os dois | Só o prunr | Só o Knip |
|---|---|---|---|
| Arquivos sem uso | 21.125 | 1.184 | 25.865 |
| Exports sem uso | 170 | 102 | 6.006 |
| Dependências sem uso | 535 | 381 | 431 |

O prunr não reporta o que está em pastas de fixtures de teste (os milhares do webpack), que o
Knip reporta como arquivos sem uso. Os 102 exports que só o prunr aponta foram conferidos um a um
(os casos e as duas limitações que eles mostram estão no
[item 23 do roadmap](roadmap/23-validation-corpus.md)); os que só o Knip aponta incluem os
exports usados no próprio arquivo, que o prunr não reporta por desenho (veja abaixo).

Para os arquivos, exports e dependências sem uso, o harness separa o que os dois apontam, o que
só o prunr aponta e o que só o Knip aponta (`results/<projeto>/diff.json`). Cada diferença vira
triagem: um falso positivo do prunr vira uma fixture e uma correção; os casos e as correções
estão no item [23 do roadmap](roadmap/23-validation-corpus.md).

Nem toda diferença é do prunr. No undici, o Knip aponta centenas de arquivos de teste que o
`borp -p "test/*.js"` roda, porque não conhece o borp. No mongoose, aponta como sem uso o
`convertToFalse` de `lib/cast/boolean.js`, que o próprio arquivo lê
(`module.exports.convertToFalse.has(value)`) e `lib/schema/boolean.js` importa. Nos projetos em
CommonJS com `module.exports = { a: fn }`, trata o objeto como um export default único e não
aponta as chaves sem uso.
