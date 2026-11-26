# 21. Caminhos citados como texto em configs montadas por função

## Objetivo

Uma configuração que um plugin reconhece, mas que é montada em tempo de execução (uma função que
devolve as suítes do Vitest, por exemplo), ainda cita os arquivos que carrega como texto:
`setupFiles: ["./packages/test-kit/src/guard/setup.ts"]`. Hoje o `static_config` não lê essas
configurações, e os arquivos aparecem como `unused-file`.

## Escopo

- Dentro de um arquivo de configuração que algum plugin já reconhece (e de quem ele importa
  localmente), uma string literal que aponta para um arquivo existente do projeto vira aresta.
- Só nesses arquivos: no resto do código, uma string que parece caminho não prova uso, e
  aceitá-la esconderia código morto.

## Critérios de aceite

- [ ] Uma fixture com uma config do Vitest montada por função, cujos `setupFiles` e
  `globalSetup` não aparecem como `unused-file`.
- [ ] Uma string com caminho num arquivo que não é de configuração continua sem criar aresta.
