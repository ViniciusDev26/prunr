# 14. Ligar e desligar plugins no `prunr.json`

## Objetivo

Cada plugin se ativa sozinho quando o `package.json` declara o pacote dele. Uma chave
`plugins` no `prunr.json` (`{ "jest": false }`) desliga um plugin, ou liga um cujo pacote não
está declarado.

## Escopo

- Só entra quando um caso real pedir: a regra do projeto é não criar opção antes da necessidade.
- Um nome de plugin desconhecido é erro, como qualquer chave desconhecida.

## Critérios de aceite

- [ ] Uma fixture com um plugin desligado e outro forçado.
