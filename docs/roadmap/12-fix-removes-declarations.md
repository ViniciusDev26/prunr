# 12. `prunr fix` apaga a declaração que fica sem uso

## Objetivo

Quando o fix tira um export, a declaração por trás dele (a função, a classe, a constante) costuma
ficar sem nenhum uso no arquivo, e outra ferramenta (o Biome) passa a acusar. O fix apaga a
declaração junto, com o comentário JSDoc dela, quando isso é seguro.

## Escopo

- **ESM:** `export function`, `export class`, `export const x = <valor>`, `export type`,
  `export interface` e `export enum` saem inteiros, em vez de só perder o `export`.
- **Referência a uma local:** `module.exports = { a }`, `exports.a = a` e `export { a }` tiram o
  export e também a declaração de `a`, quando nada mais no arquivo a lê.
- **Só o que não tem efeito colateral:** uma constante cujo valor é um literal, uma função, uma
  classe ou um objeto/array desses. `const x = criar()` perde só o export, porque apagar mudaria
  o que o módulo faz ao carregar.
- **Declaração compartilhada:** `export const a = 1, b = 2` e uma local exportada com dois nomes
  só saem quando todos os nomes saem.
- O comentário colado na declaração (JSDoc na linha de cima) sai junto; um comentário separado
  por uma linha em branco (o cabeçalho do arquivo) fica.

## Critérios de aceite

- [ ] Uma fixture com cada forma, comparando o arquivo antes e depois.
- [ ] Uma declaração com efeito colateral e uma compartilhada com um nome usado ficam.
- [ ] Depois do fix, o Biome (`noUnusedVariables`) não acusa nada que o fix criou.
