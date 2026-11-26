# 33. Triagem do que só o Knip aponta

## Objetivo

No [benchmark](../benchmark.md), o Knip aponta 26 mil arquivos, 6 mil exports e 466 dependências
que o prunr não aponta. Um corte automático por caminho mostra que quase tudo é desenho (pastas
de fixtures de teste, exports usados no próprio arquivo) ou falso positivo do Knip (arquivos de
teste de runners que ele não conhece, tabelas de constantes lidas por chave). Este item fecha o
número: cada achado que só o Knip aponta ganha uma categoria e uma evidência conferida no disco,
e o que for código morto que o prunr deixa passar vira teste e correção.

## Escopo

- O `corpus/triage.mjs` passa a classificar também os arquivos, exports e dependências que só o
  Knip aponta, com a evidência de cada categoria (o padrão de fixture, o runner que roda o
  arquivo, o acesso por chave, o uso no próprio arquivo).
- O que sobrar sem categoria é conferido um a um. Cada caso em que o prunr erra vira fixture e
  correção; cada caso em que o Knip erra fica no benchmark.

## Critérios de aceite

- [ ] Nenhum achado que só o Knip aponta fica sem categoria.
- [ ] O benchmark mostra a tabela de categorias.
