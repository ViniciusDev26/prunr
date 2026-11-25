# 18. Dependências exigidas como peer

## Objetivo

Um pacote declarado só porque outro pacote usado o exige em `peerDependencies` (o `@tiptap/pm`
do `@tiptap/react`, o `vite` do `@vitejs/plugin-react`) não é `unused-dependency`.

## Escopo

- Os `peerDependencies` são lidos do `package.json` instalado em `node_modules`, primeiro no do
  pacote do monorepo e depois no da raiz, na mesma leitura que acha os `bin` dos scripts.
- O uso é transitivo e do mesmo tipo: o peer de um pacote usado em produção é usado em produção;
  o de um pacote só de teste é só de teste (e vira `misplaced-dependency` se estiver em
  `dependencies`).
- Os peers opcionais (`peerDependenciesMeta`) contam também: o prunr prefere deixar passar uma
  dependência morta a reportar uma viva.

## Critérios de aceite

- [x] A fixture `peer-dependencies` cobre o peer direto, o transitivo, o de um pacote só de teste
  e o de um pacote sem uso, que continua sem uso.
