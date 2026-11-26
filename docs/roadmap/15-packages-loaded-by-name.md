# 15. Pacotes carregados pelo nome por outra lib

## Objetivo

Algumas libs carregam pacotes a partir de strings no código, e não de um `require`:
`new ChartJSNodeCanvas({ plugins: { modern: ["chartjs-plugin-annotation"] } })` faz o `require`
do plugin por dentro. Hoje esses pacotes vão em `dependencies.ignore`.

## Escopo

- Um plugin por lib (o primeiro seria o `chartjs-node-canvas`), que lê as strings nos argumentos
  da chamada e conta como uso do pacote.
- Exige que um plugin olhe o código, e não só arquivos de configuração: a `Contribution` ganha
  esse caminho com o primeiro plugin que precisar dele.
- Vale a pena quando o padrão se repetir; para um caso isolado, `dependencies.ignore` resolve.

## Feito

Os fatos de cada arquivo guardam as strings passadas a um binding importado de um pacote, com a
chave da propriedade em que cada uma está. A `Contribution` ganhou `loaded_by_name`: o par
(biblioteca, chave) cujas strings são pacotes que a biblioteca carrega. O primeiro plugin é o do
`chartjs-node-canvas`.

## Critérios de aceite

- [x] Uma fixture com `chartjs-node-canvas` sem `dependencies.ignore`.
