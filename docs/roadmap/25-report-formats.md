# 25. SARIF, Code Climate e Markdown

## Objetivo

Os achados nos lugares onde os times já olham: code scanning, o Code Quality do GitLab e
comentários de PR.

## Feito

- `--sarif`: SARIF 2.1.0, uma regra por tipo de achado.
- `--codeclimate`: o relatório de Code Quality do GitLab, com fingerprint estável.
- `--markdown`: uma tabela para comentar no PR.

Os três usam as posições do `--github` e combinam com `--since`, o baseline e `--production`.
