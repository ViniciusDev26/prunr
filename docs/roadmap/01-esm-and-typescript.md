# 1. ESM e TypeScript

## Objetivo

O prunr analisa projetos com `import`/`export` e TypeScript, além de CommonJS, no mesmo grafo:
um arquivo ESM pode importar um CommonJS e vice-versa.

## Escopo

- **Arquivos:** `.js`, `.cjs`, `.mjs`, `.jsx`, `.ts`, `.cts`, `.mts` e `.tsx`. Arquivos `.d.ts`
  ficam fora: só declaram tipos.
- **Imports:** `import x from`, `import { a, b as c } from`, `import * as ns from`,
  `import "./x"`, `import type`, `import()` com especificador estático e `import x = require()`.
  O uso de cada binding é rastreado como no `require` (membros lidos, chamada, uso inteiro), e um
  binding nunca referenciado é `unused-import`.
- **Exports:** `export const/let/function/class`, `export default`, `export { a, b as c }`,
  `export type`/`interface`/`enum`, `export =`. Os métodos estáticos de uma classe exportada
  também são exports, como no CommonJS. Tipos exportados sem uso são `unused-export`.
- **Reexports:** `export { a } from`, `export * from` e `export * as ns from` repassam o uso para
  o arquivo de origem. O export morto é reportado onde é definido, não no barrel.
- **Interop:** o `default` importado de um arquivo CommonJS é o `module.exports`, como no Node.
- **Resolução:** extensões de TypeScript, `.js` no especificador apontando para o `.ts`
  (`./a.js` → `./a.ts`), `paths` do `tsconfig.json` e `imports` (`#interno`) do `package.json`.
- **Tipos de pacotes:** `@types/x` conta como usado quando `x` é usado; `@types/node` conta como
  usado quando o projeto tem TypeScript ou usa um módulo do Node. Importar `x` com só o `@types/x`
  declarado não é `missing-dependency`.
- **Ciclos:** imports só de tipo (`import type`) não formam `circular-import`, porque somem na
  compilação.

## Critérios de aceite

- [x] Uma fixture por forma de import e de export da lista acima.
- [x] Uma fixture mista: ESM importando CommonJS e CommonJS fazendo `require` de ESM.
- [x] Uma fixture TypeScript com alias de `paths` e import `./x.js` de um `.ts`.
- [x] Um barrel com `export *` não mantém vivos os exports que ninguém usa através dele.
- [x] `prunr init` propõe entradas e globs de teste num projeto TypeScript.
