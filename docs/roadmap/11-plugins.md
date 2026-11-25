# 11. Plugins internos

## Objetivo

Cada tecnologia JS/TS que o prunr entende (Jest, Vitest e, depois, Next.js, NestJS, Vue...) fica
num plugin próprio, separado do núcleo que lê arquivos, monta o grafo e roda os checks. Dar
suporte a uma tecnologia nova é escrever um plugin, sem mexer no resto.

## Escopo

- **Compilados no binário**, não carregados em tempo de execução: um binário estático, sem ABI
  para manter nem código de terceiros rodando. Se um dia fizer sentido aceitar plugins de fora,
  o trait é a fronteira natural.
- **O trait `Plugin`** (`src/plugins/mod.rs`): `name` (o pacote que o ativa), `enabled` (por
  padrão, o pacote está declarado no `package.json`) e `contribute`, que acrescenta a uma
  `Contribution` o que a ferramenta roda e carrega.
- **Um plugin só lê arquivos**: configuração lida estaticamente (`static_config`), nunca
  executada.
- **A `Contribution` cresce com os plugins**: hoje ela tem arquivos de teste, arquivos carregados
  e pacotes citados. Entradas de produção por convenção e exports lidos pelo framework entram
  com o primeiro plugin que precisar deles ([item 9](09-next-and-nest.md)), não antes.
- **Ligar e desligar pelo `prunr.json`** (`"plugins": { "jest": false }`) só entra quando um
  caso real pedir: hoje cada plugin se ativa sozinho pelo `package.json`.

## Critérios de aceite

- [x] Jest e Vitest são os dois primeiros plugins, sem mudança de comportamento: os testes e as
      fixtures que já existiam passam sem alteração na saída.
- [x] O registro (`PLUGINS`) lista os plugins numa ordem estável, e um teste fixa essa lista.
- [x] O CLAUDE.md diz como adicionar um plugin.
