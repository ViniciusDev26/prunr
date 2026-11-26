# 37. Binários não declarados

## Objetivo

Um script do `package.json` que roda `eslint` sem o pacote declarado depende do que estiver
instalado globalmente ou de outro pacote que o traga. O prunr já lê os binários dos scripts para
saber que um pacote é usado; falta o contrário.

## Escopo

- Um achado `missing-binary` (`script: binário`) para o comando de um script que nenhum pacote
  declarado fornece, mas um pacote instalado em `node_modules` fornece.
- Um comando que nenhum pacote instalado fornece é do sistema (`rm`, `git`, `docker`) e nunca é
  reportado.

## Feito

`missing-binary` aponta o binário que um script roda e que só um pacote instalado mas não
declarado fornece. Em vez de uma lista de comandos do sistema, a regra é exata: o binário que
nenhum pacote de `node_modules` fornece é do sistema. No corpus, 9 achados, todos reais (o `c8`
do fastify, o `eslint` do node-semver e do excalidraw, que vêm de outra dependência).

## Critérios de aceite

- [x] Uma fixture com um script que roda um binário declarado, um não declarado e um do sistema.
