# 10. Performance

## Objetivo

Manter o check rápido em projetos grandes. Só começa quando houver um projeto real lento o
bastante para medir.

## Escopo

- Ler e analisar os arquivos em paralelo (`rayon`): a análise de cada arquivo é independente.
- Cache por hash do conteúdo dos fatos de cada arquivo, numa pasta ignorada pelo git.
- Um benchmark reproduzível (um projeto gerado com milhares de arquivos) antes de otimizar.

## Critérios de aceite

- [ ] O benchmark mostra o ganho de cada otimização.
- [ ] A saída é idêntica com e sem cache, e com qualquer número de threads.
