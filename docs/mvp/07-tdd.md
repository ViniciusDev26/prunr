# 7. Todo o MVP construído com TDD

## Objetivo

Nenhum comportamento entra no prunr sem um teste que falhou antes. Para um detector de código
morto isso é essencial: um falso positivo leva alguém a apagar código vivo.

## Regras

1. **Vermelho → verde → refatorar.** Primeiro o teste que falha pelo motivo certo, depois o mínimo
   de código para passar, depois a limpeza com os testes verdes.
2. **Todo bug vira teste antes da correção.** Inclui as divergências encontradas no teste de
   aceite com o projeto de referência.
3. **No histórico, o teste vem antes.** O teste entra num commit `test:` antes do `feat:`/`fix:`
   que o faz passar, ou no mesmo commit quando a mudança é pequena. A revisão do PR confere isso.

## Estrutura de testes

- **Unitários** junto de cada módulo (parser, resolver, grafo, baseline, config).
- **Fixtures** em `tests/fixtures/<caso>/`: um mini projeto CommonJS por comportamento, com
  `package.json`, `prunr.json` e a saída esperada escrita no próprio teste (`tests/check.rs`).
- **Teste de aceite** contra um projeto CommonJS real de referência, rodado localmente, já que
  ele é privado e fica fora do CI público.

## Critérios de aceite

- [x] Cada critério de aceite dos pontos 1–6 tem pelo menos um teste automatizado correspondente
      (os do ponto 6 são os jobs do workflow `release`).
- [x] Cada linha da tabela de formas CommonJS ([ponto 1](01-single-graph.md)) tem fixture própria.
- [x] **Cobertura de 100%** (linhas e funções) medida com `cargo llvm-cov` e exigida no CI:
      abaixo de 100% o CI falha.
- [x] O histórico mostra testes entrando antes das implementações ou junto delas. Com squash
      merge, essa ordem fica nos commits de cada PR, e o commit da `main` lista as mensagens
      deles na ordem em que entraram.
