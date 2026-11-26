# 14. Ligar e desligar plugins no `prunr.json`

## Objetivo

Cada plugin se ativa sozinho quando o `package.json` declara o pacote dele. Uma chave
`plugins` no `prunr.json` (`{ "jest": false }`) desliga um plugin, ou liga um cujo pacote não
está declarado.

## Escopo

- Só entra quando um caso real pedir: a regra do projeto é não criar opção antes da necessidade.
- Um nome de plugin desconhecido é erro, como qualquer chave desconhecida.

## Feito

`"plugins": { "jest": false, "mocha": true }` no `prunr.json` desliga e liga plugins pelo nome,
em todos os pacotes do repositório, e um nome desconhecido é erro. O `prunr.json` e o
`schema.json` publicado ganharam a chave.

## Critérios de aceite

- [x] Uma fixture com um plugin desligado e outro forçado.
