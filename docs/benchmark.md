# Benchmark

O prunr e o Knip nos 55 projetos do [corpus de validação](../corpus/README.md), no mesmo commit
de cada projeto, com as dependências instaladas. Os números são de uma máquina Linux (x86-64,
32 núcleos), com o prunr de release do commit `76a3c56` e o Knip 5 rodado por `npx`, um
projeto por vez (`--jobs 1`). O tempo do Knip inclui a partida do Node.

Para reproduzir: `cargo build --release && node corpus/run.mjs --install --knip --jobs 1`.

## Tempo

Os 44 projetos em que os dois rodaram, do mais lento para o Knip:

| Projeto | prunr | Knip |
|---|---|---|
| webpack | 0,94 s | 6,83 s |
| date-fns | 0,09 s | 4,80 s |
| mongoose | 0,03 s | 3,03 s |
| rollup | 0,31 s | 2,89 s |
| cli | 0,10 s | 2,84 s |
| fastify | 0,02 s | 2,71 s |
| bullmq | 0,03 s | 2,53 s |
| undici | 0,03 s | 2,28 s |
| pino | 0,01 s | 2,24 s |
| mocha | 0,02 s | 2,22 s |
| got | 0,01 s | 2,17 s |
| axios | 0,03 s | 2,04 s |
| socket.io | 0,04 s | 2,03 s |
| cheerio | 0,01 s | 2,00 s |
| jsdom | 0,03 s | 1,97 s |
| knex | 0,03 s | 1,96 s |
| execa | 0,02 s | 1,95 s |
| nodemailer | 0,02 s | 1,94 s |
| commander | 0,01 s | 1,81 s |
| pm2 | 0,03 s | 1,76 s |
| lodash | 0,02 s | 1,67 s |
| ioredis | 0,03 s | 1,67 s |
| yargs | 0,01 s | 1,65 s |
| body-parser | 0,01 s | 1,64 s |
| express | 0,01 s | 1,62 s |
| hapi | 0,01 s | 1,61 s |
| node-semver | 0,01 s | 1,57 s |
| ws | 0,01 s | 1,56 s |
| debug | 0,01 s | 1,55 s |
| multer | 0,01 s | 1,54 s |
| moment | 0,02 s | 1,53 s |
| winston | 0,01 s | 1,53 s |
| helmet | 0,01 s | 1,52 s |
| ajv | 0,01 s | 1,50 s |
| chalk | 0,01 s | 1,46 s |
| dotenv | 0,01 s | 1,46 s |
| node-red | 0,02 s | 1,45 s |
| koa | 0,01 s | 1,45 s |
| node-jsonwebtoken | 0,01 s | 1,44 s |
| nodemon | 0,01 s | 1,41 s |
| gulp | 0,01 s | 1,40 s |
| json-server | 0,01 s | 1,35 s |
| node-glob | 0,01 s | 1,33 s |
| passport | 0,01 s | 1,31 s |

O webpack tem mais de 24 mil arquivos-fonte. Num monorepo privado de 13 mil arquivos, fora do
corpus, o check leva 0,6 s.

## Onde só um dos dois roda

O prunr rodou nos 55. O Knip carrega as configurações das ferramentas executando-as, e em 5
projetos (ESLint, Vite, tRPC, TanStack Query, Excalidraw) parou num erro ao carregar uma delas.
Em outros 6 (Prettier, Nest, Jest, Sequelize, TypeORM, Zod) a instalação das dependências falhou
(conflito de peers, versões antigas do npm), e o Knip não roda sem elas.

## Concordância

Nos 44 projetos em que os dois rodaram, somando todos:

| | Os dois | Só o prunr | Só o Knip |
|---|---|---|---|
| Arquivos sem uso | 26.506 | 738 | 16.699 |
| Exports sem uso | 103 | 58 | 1.443 |
| Dependências sem uso | 326 | 208 | 337 |

O webpack, com fixtures de teste aos milhares, domina a linha dos arquivos. Os 58 exports que só o
prunr aponta foram conferidos um a um e se sustentam (os casos estão no
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
