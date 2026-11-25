# 8. `require` dinâmico com padrão

## Objetivo

Um `require` com template literal, como `` require(`./handlers/${name}`) ``, aponta para todos
os arquivos que o padrão pode alcançar, em vez de virar só um `unresolved-require`.

## Escopo

- O template vira um glob: cada `${...}` vira `*`, e o prefixo antes do primeiro `${` precisa ser
  um caminho relativo (`./` ou `../`). `` `./handlers/${a}/${b}.js` `` → `./handlers/*/*.js`.
- Cada arquivo do projeto que o glob alcança é usado inteiro (todos os seus exports), como num
  `require` cujo resultado escapa.
- Vale também para concatenação simples: `require("./handlers/" + name)`.
- Um padrão sem prefixo relativo, ou que não alcança nenhum arquivo, continua sendo
  `unresolved-require`.
- O mesmo vale para `import()` quando o item 1 entrar.

## Critérios de aceite

- [x] Uma fixture com template literal e com concatenação, alcançando arquivos em subpastas.
- [x] Um padrão que não alcança nada continua como aviso.
