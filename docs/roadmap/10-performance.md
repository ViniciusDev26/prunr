# 10. Performance

## Objetivo

Manter o check rápido em projetos grandes.

## Feito

Num monorepo de referência com 13 mil arquivos, o check levava 1,35 s numa thread só. Medindo
por fase, o parse custava pouco; o tempo estava em resolver os imports de cada arquivo. Os dois
passos por arquivo agora rodam em paralelo (`rayon`), e o check caiu para 0,61 s:

- A leitura e a análise de cada arquivo são independentes.
- A resolução dos imports de cada arquivo também: o resolver é compartilhado entre as threads, e
  o cache dele vale para todas.
- Os resultados voltam na ordem dos arquivos, então a saída e o erro de sintaxe reportado não
  dependem do número de threads.

## Depois

Só quando um projeto real ficar lento de novo:

- Os checks e o relatório (cerca de um terço do tempo que sobrou) e a montagem dos workspaces.
- Cache por hash do conteúdo dos fatos de cada arquivo, numa pasta ignorada pelo git.
- Um benchmark reproduzível (um projeto gerado com milhares de arquivos).

## Critérios de aceite

- [x] A saída é idêntica com qualquer número de threads (`RAYON_NUM_THREADS=1`).
- [x] O erro de sintaxe reportado é sempre o do primeiro arquivo.
- [ ] O benchmark mostra o ganho de cada otimização.
- [ ] A saída é idêntica com e sem cache.
