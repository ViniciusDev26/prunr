# Configuração

Crie um `prunr.json` na raiz do projeto (`prunr init` gera um primeiro, a partir do
`package.json`) e rode `prunr check` nela.

```json
{
  "$schema": "https://unpkg.com/@prunr/prunr/schema.json",
  "entries": {
    "production": ["src/index.js", "bin/cli.js"],
    "test": ["test/**", "**/*.test.js"]
  },
  "dependencies": {
    "ignore": ["eslint-plugin-import"]
  },
  "baseline": ".prunr-baseline.json"
}
```

## Opções

| Opção | Obrigatória | O que é |
|---|---|---|
| `entries.production` | sim | Caminhos ou globs dos arquivos por onde o programa começa. Tudo o que eles alcançam, direta ou indiretamente, está vivo. Com Vite, Next.js ou NestJS, pode ficar vazio (`[]`): veja [Frameworks](frameworks.md). |
| `$schema` | não | O JSON Schema do arquivo, para autocompletar e validar no editor. |
| `entries.test` | não | Caminhos ou globs do código de teste, incluindo helpers e mocks. Código de teste nunca é reportado, e o que só ele alcança está morto em produção. Com Jest ou Vitest, liste só o que o runner não acha sozinho. |
| `dependencies.ignore` | não | Pacotes que ficam de fora das verificações de dependência: os carregados de um jeito que nem o grafo nem os `scripts` mostram (um plugin citado no arquivo de configuração de outra ferramenta). |
| `baseline` | não | Arquivo de baseline, relativo à raiz. Veja [CI e código legado](ci.md#adotando-num-código-legado). |

Caminhos e globs são relativos à raiz; num glob, `*` não atravessa `/` e `**` atravessa. Cada
entrada precisa casar com pelo menos um arquivo-fonte (`.js`, `.cjs`, `.mjs`, `.jsx`, `.ts`, `.cts`,
`.mts` ou `.tsx`).

Uma opção só entra no arquivo quando alguma funcionalidade precisa dela. Uma chave desconhecida é
erro, para que um erro de digitação não passe em silêncio.

## Os `scripts` do `package.json`

Os `scripts` também usam pacotes, e o prunr os lê:

- o binário de cada comando (`jest`, `tsc`, `npx nodemon`, `cross-env NODE_ENV=test jest`,
  `dotenv -- jest`) conta como uso do pacote que o declara no campo `bin` (lido em
  `node_modules`; sem ele, vale o nome do pacote);
- `node -r dotenv/config` ou `--import tsx` conta como uso do pacote carregado;
- um caminho dentro de `node_modules` (`node node_modules/jest/bin/jest.js`) conta como uso do
  pacote dele.

Um pacote usado por script não é `unused-dependency` nem `misplaced-dependency`, e um binário sem
pacote declarado (`docker`, `git`) não é `missing-dependency`.

## Códigos de saída

| Código | Significado |
|---|---|
| `0` | Nenhum achado, ou só avisos. |
| `1` | Há achados novos, ou entradas obsoletas no baseline. |
| `2` | O comando não pôde rodar: `prunr.json` ausente ou inválido, `package.json` inválido, arquivo com erro de sintaxe ou argumento desconhecido. |
