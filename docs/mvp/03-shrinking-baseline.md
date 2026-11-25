# 3. Baseline que só diminui

## Objetivo

Permitir adotar o prunr num código legado sem precisar limpar tudo antes. Os achados que já
existem ficam registrados, só um achado **novo** quebra o CI, e a lista só pode diminuir, em
todas as verificações.

## Comportamento

- O arquivo de baseline é indicado no config (`"baseline": ".prunr-baseline.json"`).
- `prunr baseline` reescreve o arquivo com os achados atuais.
- `prunr check`:
  - achado ausente no baseline → **novo** → falha;
  - entrada do baseline que deixou de ser achada → **obsoleta** → falha, pedindo para rodar
    `prunr baseline` (a lista só diminui);
  - achado presente no baseline → ignorado.
- Identidade de um achado: `tipo + arquivo + símbolo` (ou `tipo + pacote`, ou o ciclo
  normalizado). **Sem número de linha**, para que mover código não invalide o baseline.
- Saída determinística: entradas ordenadas e JSON estável, para gerar diffs limpos em PR.

### Decisões tomadas na implementação

- **O baseline guarda só erros.** Avisos (`unresolved-require`) já não falham o check.
- **Um baseline configurado que ainda não existe conta como vazio**: o primeiro `prunr check`
  mostra tudo, e `prunr baseline` cria o arquivo.
- **Entradas obsoletas** aparecem como `stale-baseline` na saída humana e em `"stale"` no JSON.

## Critérios de aceite

- [x] Achado novo fora do baseline → código de saída `1`, listado como novo.
- [x] Entrada obsoleta no baseline → código de saída `1`, listada como obsoleta.
- [x] Mover um export de linha não altera o resultado.
- [x] Ciclo `a → b → c → a` e o mesmo ciclo encontrado começando por `b` geram a mesma entrada.
- [x] Rodar `prunr baseline` duas vezes seguidas produz arquivos byte a byte idênticos.
- [x] Sem baseline configurado, todo achado conta como novo.
