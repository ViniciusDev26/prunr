# 22. Um pacote de cada vez num monorepo grande

## Objetivo

Adotar o prunr num monorepo com dezenas de pacotes e milhares de achados sem ter que ler tudo de
uma vez: olhar um pacote, resolver, passar ao próximo.

## Escopo

- `prunr check --workspace <pasta>`: o grafo continua sendo o do repositório inteiro (um export
  só é morto se nenhum pacote o usa), mas só os achados daquele pacote são mostrados e contam
  para o código de saída.
- Um resumo com a contagem por tipo de achado e por pacote, para escolher por onde começar.
- Complementa o baseline, que já permite adotar sem resolver tudo.

## Critérios de aceite

- [ ] Numa fixture de monorepo, `--workspace` mostra só os achados do pacote, e um export usado
  por outro pacote continua vivo.
- [ ] O resumo soma o mesmo que a saída completa.
