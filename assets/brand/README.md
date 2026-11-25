# Identidade visual do prunr

O prunr poda o grafo de módulos de um projeto. A marca é esse grafo: uma raiz, dois módulos e
quatro folhas, das quais uma é código morto. A aresta até ela leva um corte (o traço coral), e o
nó fica vazado e apagado. O que está vivo é verde.

| Arquivo | Uso |
| --- | --- |
| [`logo.svg`](logo.svg) / [`logo-dark.svg`](logo-dark.svg) | Marca + nome, em fundo claro / escuro (README, site, docs) |
| [`mark.svg`](mark.svg) / [`mark-dark.svg`](mark-dark.svg) | Só o símbolo, sem fundo |
| [`wordmark.svg`](wordmark.svg) / [`wordmark-dark.svg`](wordmark-dark.svg) | Só o nome |
| [`icon.svg`](icon.svg), [`icon-512.png`](icon-512.png) | Ícone quadrado com fundo (avatar da organização no GitHub e no npm, favicon) |
| [`social-preview.svg`](social-preview.svg), [`social-preview.png`](social-preview.png) | Prévia social do repositório (GitHub → Settings → Social preview), 1280×640 |

Todos os SVGs saem do [`generate.py`](generate.py): mude a geometria ou as cores lá e rode
`python3 assets/brand/generate.py`. Os PNGs são exportados com `rsvg-convert`:

```sh
rsvg-convert -w 512 assets/brand/icon.svg -o assets/brand/icon-512.png
rsvg-convert -w 1280 -h 640 assets/brand/social-preview.svg -o assets/brand/social-preview.png
```

## Cores

| Papel | Fundo claro | Fundo escuro |
| --- | --- | --- |
| Nó vivo | `#1FA463` | `#3DDC84` |
| Aresta viva | `#2F4A3B` | `#9FB8A9` |
| Código morto | `#A7B3AC` | `#56665D` |
| Corte | `#F05A4F` | `#FF6B5E` |
| Texto | `#10231A` | `#ECF3EE` |
| Fundo do ícone | | `#10231A` |

## Nome e tipografia

- O nome se escreve sempre em minúsculas: **prunr**, inclusive no início de frase.
- O wordmark é desenhado em traço único (paths, sem fonte), com cantos arredondados. Não o
  substitua por texto numa fonte qualquer.
- Textos de apoio (slogans, prévia social) usam fonte monoespaçada: JetBrains Mono, ou a
  `ui-monospace` do sistema.

## Uso

- Em fundo escuro, use as variantes `-dark`; não inverta as cores do arquivo claro.
- Deixe ao redor da marca uma margem livre de pelo menos a altura de um nó folha.
- Abaixo de 24 px use só o `icon.svg`; o wordmark perde a leitura.
- Não gire, não distorça e não troque o verde do vivo pelo coral do corte: as cores carregam o
  significado (vivo × podado).
