# 21. Caminhos citados como texto em configs montadas por função

## Objetivo

Uma configuração que um plugin reconhece, mas que é montada em tempo de execução (uma função que
devolve as suítes do Vitest, por exemplo), ainda cita os arquivos que carrega como texto:
`setupFiles: ["./packages/test-kit/src/guard/setup.ts"]`. Hoje o `static_config` não lê essas
configurações, e os arquivos aparecem como `unused-file`.

## Escopo

- Dentro de um arquivo de configuração que algum plugin já reconhece (e dos que ele importa
  diretamente), uma string literal que aponta para um arquivo existente do projeto vira aresta.
- Só nesses arquivos: no resto do código, uma string que parece caminho não prova uso, e
  aceitá-la esconderia código morto.

## Feito

Cada caminho relativo escrito numa config que um plugin reconhece, ou num arquivo que ela importa
diretamente, que aponta para um arquivo do projeto vira aresta e entrada de teste, como os
`setupFiles` lidos de uma config estática. A primeira versão seguia os imports da config até o
fim, e no corpus a config do ESLint, que importa o próprio ESLint, levava a lib inteira junto;
parando no primeiro import, 32 arquivos do corpus deixam de ser `unused-file`, todos usos reais
(`globalSetup` do Vitest, aliases do Vite, `rollupOptions.input` dos playgrounds do Vite).

## Critérios de aceite

- [x] Uma fixture com uma config do Vitest montada por função, cujos `setupFiles` e
  `globalSetup` não aparecem como `unused-file`.
- [x] Uma string com caminho num arquivo que não é de configuração continua sem criar aresta.
