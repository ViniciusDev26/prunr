# CLAUDE.md

Instruções para agentes (e humanos) que trabalham no prunr.

## O projeto

prunr é um detector de código morto para Node.js, com CommonJS como cidadão de primeira classe,
escrito em Rust. O escopo do MVP e os critérios de aceite estão em [`docs/mvp/`](docs/mvp/);
leia o ponto relevante antes de implementar qualquer coisa.

## TDD é obrigatório

Nenhum comportamento entra no prunr sem um teste que falhou antes. Um detector de código morto
com falso positivo leva alguém a apagar código vivo, então isso não é negociável.

1. **Vermelho → verde → refatorar.**
   - Escreva primeiro o teste e rode para vê-lo **falhar pelo motivo certo** (não por erro de
     compilação nem de digitação).
   - Escreva o **mínimo** de código para o teste passar.
   - Refatore só com todos os testes verdes.
2. **Todo bug vira teste antes da correção.** Primeiro um teste que reproduz o bug e falha, depois
   a correção. Vale também para divergências encontradas no teste de aceite com o projeto de referência.
3. **No histórico, o teste vem antes.** O teste entra num commit `test:` antes do `feat:`/`fix:`
   que o faz passar, ou no mesmo commit quando a mudança é pequena. Nunca depois.
4. **Não escreva código "para depois".** Código sem teste que o exija não deve existir.

## Cobertura de testes: 100%

**A cobertura exigida é de 100% de linhas e de funções, sem exceções.**

- Medida com `cargo llvm-cov`. O CI roda
  `cargo llvm-cov --fail-under-lines 100 --fail-under-functions 100` e falha abaixo disso.
- Rode o mesmo comando localmente antes de abrir um PR.
- É proibido excluir código da medição para bater a meta (atributos de exclusão, filtros de
  arquivo no `llvm-cov`, `#[cfg(not(coverage))]` etc.).
- Se um trecho não dá para cobrir com teste, o problema é o design: extraia a parte com efeito
  colateral (I/O, processo, ambiente) para trás de uma interface pequena e teste a lógica
  separadamente. Se mesmo assim não houver como testar, o trecho provavelmente não deveria existir.
- Cobertura de 100% é o piso, não a prova de qualidade: um teste precisa afirmar o comportamento,
  não só executar a linha.

## Estrutura de testes

- **Unitários** junto de cada módulo (parser, resolver, grafo, baseline, config).
- **Fixtures** em `tests/fixtures/<caso>/`: um mini projeto CommonJS por comportamento, com
  `package.json`, `prunr.json` e a saída esperada comparada por snapshot (`insta`). Um
  comportamento novo do grafo começa por uma fixture nova.
- **Teste de aceite** contra um projeto CommonJS real de referência, rodado localmente (privado,
  fora do CI).

## Commits

- Conventional Commits (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`, `chore:`). O
  release-please usa isso para gerar a versão e o CHANGELOG ([ponto 6](docs/mvp/06-ci-and-auto-release.md)).
- Antes de cada commit: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` e a cobertura de 100%.
