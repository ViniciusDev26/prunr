# 24. `prunr why`

## Objetivo

Antes de apagar o que a ferramenta diz que está morto, ou de manter o que ela diz que está vivo,
ver o porquê, com o mesmo grafo do check.

## Feito

- `prunr why <arquivo>`: a cadeia mais curta de imports desde uma entrada (de produção, ou de
  teste), com a linha de cada import; quem importa o arquivo quando nada o alcança.
- `prunr why <arquivo>:<export>`: os arquivos que usam o export, marcados `production`, `test`
  ou `unreached`.
- `prunr why <pacote>`: os arquivos que o importam, os scripts que o rodam, as ferramentas e
  frameworks que o carregam e os pacotes que o exigem como peer.
