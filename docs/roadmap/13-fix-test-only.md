# 13. `prunr fix --test-only`

## Objetivo

O que só os testes usam é código morto em produção, mas o fix não toca nele: apagar a função
quebraria os testes que a importam, e apagar testes sozinho é arriscado. Este modo faz a parte
segura e deixa a decisão com quem revisa.

## Escopo

- `prunr fix --test-only` apaga o código de produção que só os testes usam (com as regras do
  [item 12](12-fix-removes-declarations.md)) e lista, por arquivo de teste, os imports que
  ficaram quebrados.
- Não apaga nem edita testes.
- Fica para quando o padrão aparecer em outro projeto além do de referência: lá, um desses
  achados era um bug (um desligamento gracioso que nunca foi ligado), e apagar teria escondido.

## Critérios de aceite

- [ ] A lista de testes a revisar aponta arquivo e linha de cada import quebrado.
- [ ] Sem `--test-only`, o fix continua sem tocar no que só os testes usam.
