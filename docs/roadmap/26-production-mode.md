# 26. Modo produção

## Objetivo

Responder se o que vai para produção está limpo, sem o ruído do código de desenvolvimento.

## Feito

`prunr check --production` mantém arquivos e exports sem uso e dependências no lugar errado; os
outros achados só nos arquivos que a produção alcança (ciclos, só com todos os arquivos de
produção); e `unused-dependency` só entre `dependencies`. O mesmo filtro vale para o baseline.
