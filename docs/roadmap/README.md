# Roadmap

O que vem depois do [MVP](../mvp/). Cada item tem um arquivo com objetivo, escopo e critérios de
aceite, e segue as mesmas regras do MVP: TDD e 100% de cobertura ([CLAUDE.md](../../CLAUDE.md)).
A numeração é a da priorização original e não muda quando um item termina.

## Feito

- [x] 1. [ESM e TypeScript](01-esm-and-typescript.md)
- [x] 2. [Dependências usadas fora do código](02-dependencies-outside-code.md)
- [x] 3. [Jest e Vitest](03-jest-and-vitest.md)
- [x] 4. [`prunr fix`](04-fix.md)
- [x] 5. [Saída para PR](05-pull-request-output.md)
- [x] 6. [Ignorar no código](06-ignore-in-code.md)
- [x] 7. [Workspaces e monorepo](07-workspaces.md)
- [x] 8. [`require` dinâmico com padrão](08-dynamic-require-patterns.md)
- [x] 9. [Next.js e NestJS](09-next-and-nest.md)
- [x] 11. [Plugins internos](11-plugins.md)
- [x] 12. [`prunr fix` apaga a declaração que fica sem uso](12-fix-removes-declarations.md)
- [x] 16. [Vite e React](16-vite-and-react.md)
- [x] 17. [Os arquivos que os scripts rodam são entradas](17-script-entries.md)
- [x] 18. [Dependências exigidas como peer](18-peer-dependencies.md)
- [x] 19. [Playwright, Drizzle Kit, PostCSS e Vitest `projects`](19-tooling-plugins.md)
- [x] 20. [Imports em folhas de estilo](20-stylesheet-imports.md)
- [x] 23. [Corpus de validação](23-validation-corpus.md)
- [x] 24. [`prunr why`](24-why.md)
- [x] 25. [SARIF, Code Climate e Markdown](25-report-formats.md)
- [x] 26. [Modo produção](26-production-mode.md)

## Depois

- [ ] 10. [Performance](10-performance.md)
- [ ] 13. [`prunr fix --test-only`](13-fix-test-only.md)
- [ ] 14. [Ligar e desligar plugins no `prunr.json`](14-toggle-plugins.md)
- [ ] 15. [Pacotes carregados pelo nome por outra lib](15-packages-loaded-by-name.md)
- [ ] 21. [Caminhos citados como texto em configs montadas por função](21-paths-in-runtime-configs.md)
- [ ] 22. [Um pacote de cada vez num monorepo grande](22-one-package-at-a-time.md)

## Decidido não fazer

- **Entradas declaradas em arquivos de deploy e CI** (`cloudbuild.yaml`, `Dockerfile`,
  workflows do GitHub). Um processo que só a infraestrutura inicia é uma entrada do projeto, e é
  para isso que existe `entries.production`: a configuração deixa explícito o que o código não
  mostra.

Antes de tudo, e sempre que um item terminar: rodar o prunr no projeto CommonJS de referência e
transformar cada divergência em teste (o teste de aceite do [MVP](../mvp/README.md)).
