# Benchmark

O prunr e o Knip nos projetos do [corpus de validação](../corpus/README.md), no mesmo commit de
cada projeto, com as dependências instaladas. Os números são de uma máquina Linux (x86-64), com
o prunr de release e o Knip 5 rodado por `npx`. O tempo do Knip inclui a partida do Node.

Para reproduzir: `cargo build --release && node corpus/run.mjs --install --knip`.

## Tempo

| Projeto | prunr | Knip |
|---|---|---|
| webpack | 0,78 s | 7,19 s |
| date-fns | 0,07 s | 4,87 s |
| mongoose | 0,02 s | 3,30 s |
| express | 0,01 s | 2,98 s |
| npm cli | 0,09 s | 2,85 s |
| fastify | 0,01 s | 2,82 s |
| pino | 0,01 s | 2,42 s |
| undici | 0,02 s | 2,37 s |
| mocha | 0,01 s | 2,35 s |
| got | 0,01 s | 2,17 s |
| axios | 0,02 s | 2,15 s |
| knex | 0,02 s | 1,99 s |
| nodemailer | 0,01 s | 1,98 s |
| commander | 0,01 s | 1,90 s |
| yargs | 0,01 s | 1,75 s |
| winston | 0,01 s | 1,55 s |
| koa | 0,01 s | 1,50 s |
| hapi | 0,01 s | 1,42 s |
| json-server | < 0,01 s | 1,37 s |

O webpack tem mais de 24 mil arquivos-fonte. Num monorepo privado de 13 mil arquivos, fora do
corpus, o check leva 0,6 s.

## Onde só um dos dois roda

O Knip carrega as configurações das ferramentas executando-as. Em 5 dos 30 projetos (ESLint,
Vite, tRPC, TanStack Query, Excalidraw) ele parou num erro ao carregar uma delas; o prunr, que
lê as configurações como texto, rodou nos 30. Em outros 6 a instalação das dependências falhou
(conflito de peers, versões antigas do npm), e o Knip não roda sem elas.

## Concordância

Para os arquivos, exports e dependências sem uso, o harness separa o que os dois apontam, o que
só o prunr aponta e o que só o Knip aponta (`results/<projeto>/diff.json`). As diferenças viram
triagem: um falso positivo do prunr vira uma fixture e uma correção. A primeira rodada achou,
entre outros, test runners que o prunr não conhecia (Mocha, `node --test`, tap), configurações do
Vitest com projetos inline e pacotes que importam a si mesmos; os casos e as correções estão no
item [23 do roadmap](roadmap/23-validation-corpus.md).

Nem toda diferença é do prunr. No undici, o Knip aponta 456 arquivos de teste que o
`borp -p "test/*.js"` roda, porque não conhece o borp; nos projetos em CommonJS com
`module.exports = { a: fn }`, o Knip trata o objeto como um export default único e não aponta as
chaves sem uso.
