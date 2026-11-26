# 33. Triagem do que só o Knip aponta

## Objetivo

No [benchmark](../benchmark.md), o Knip aponta 26 mil arquivos, 6 mil exports e 466 dependências
que o prunr não aponta. Um corte automático por caminho mostra que quase tudo é desenho (pastas
de fixtures de teste, exports usados no próprio arquivo) ou falso positivo do Knip (arquivos de
teste de runners que ele não conhece, tabelas de constantes lidas por chave). Este item fecha o
número: cada achado que só o Knip aponta ganha uma categoria e uma evidência conferida no disco,
e o que for código morto que o prunr deixa passar vira teste e correção.

## Escopo

- Um script do corpus classifica também os arquivos, exports e dependências que só o
  Knip aponta, com a evidência de cada categoria (o padrão de fixture, o runner que roda o
  arquivo, o acesso por chave, o uso no próprio arquivo).
- O que sobrar sem categoria é conferido um a um. Cada caso em que o prunr erra vira fixture e
  correção; cada caso em que o Knip erra fica no benchmark.

## Feito

[`corpus/knip-triage.mjs`](../../corpus/knip-triage.mjs) classifica cada achado que só o Knip
aponta com o `prunr why` dele e a evidência no disco; a tabela está no
[benchmark](../benchmark.md#o-que-só-o-knip-aponta). Dos 26 mil arquivos, 25,3 mil são
fixtures e arquivos de teste; dos 6 mil exports, 3,8 mil são de módulos usados inteiros (as
tabelas de constantes do mysql) e mil são usados no próprio arquivo.

Os erros do prunr que a triagem achou, cada um com teste e correção:

- o valor do `--watch-files` (e das outras opções com valor) do Mocha era lido como spec, e todo
  o `lib/` do mongoose virava teste;
- uma dependência da raiz contava como usada por um pacote que a declara ele mesmo (as do
  arborist no npm CLI); a correção mostrou que um `tsconfig.json` não contava como uso do
  `typescript`, e cerca de 35 falsos positivos sumiram;
- o `prunr why` de um export não seguia reexports nem dizia quando o export é de uma entrada ou
  usado no próprio arquivo, e respondia "used by no file" para um export vivo.

## Critérios de aceite

- [x] Nenhum achado que só o Knip aponta fica sem categoria.
- [x] O benchmark mostra a tabela de categorias.
