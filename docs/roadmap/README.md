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
- [x] 27. [Plugin de TypeScript](27-typescript-plugin.md)
- [x] 28. [Vue, Svelte, Astro e MDX](28-single-file-components.md)
- [x] 29. [Plugins declarativos de ferramentas](29-tool-plugins.md)
- [x] 30. [SvelteKit e Nuxt](30-sveltekit-and-nuxt.md)
- [x] 31. [Astro e React Router/Remix](31-astro-and-react-router.md)
- [x] 32. [VitePress](32-vitepress.md)
- [x] 33. [Triagem do que só o Knip aponta](33-knip-only-triage.md)

## Fase 1: credibilidade (concluída)

O plano era provar que o prunr acerta e que é rápido, antes de ampliar a cobertura:

- [x] Corpus de validação com 100 projetos e triagem dos achados ([23](23-validation-corpus.md)).
- [x] Benchmark contra o Knip ([`benchmark.md`](../benchmark.md)).
- [x] `prunr why` ([24](24-why.md)).
- [x] SARIF, Code Climate e Markdown ([25](25-report-formats.md)).
- [x] Modo produção ([26](26-production-mode.md)).

O que a triagem deixou para depois depende da fase 2 (o plugin de TypeScript, os compiladores de
Vue, Svelte e Angular, os aliases de Jest e Vite) e está no item 23.

## Fase 2: cobertura

Os pacotes e arquivos que as ferramentas carregam sem import:

- [x] Plugin de TypeScript ([27](27-typescript-plugin.md)).
- [x] `.vue`, `.svelte`, `.astro` e `.mdx`: o script de cada um passa pelo parser ([28](28-single-file-components.md)).
- [x] Plugins declarativos para ferramentas: ESLint, Prettier, Babel, commitlint, Tailwind,
  Storybook, Cypress, husky, lint-staged, Prisma, tsup, Rollup e webpack
  ([29](29-tool-plugins.md)).
- [x] SvelteKit e Nuxt ([30](30-sveltekit-and-nuxt.md)).
- [x] Astro e React Router/Remix ([31](31-astro-and-react-router.md)).
- [x] VitePress ([32](32-vitepress.md)).

## Fase 3: profundidade

A fase 2 ampliou o que o prunr enxerga; a 3 amplia o que ele reporta e quem o adota, na ordem:

1. [x] Triagem do que só o Knip aponta ([33](33-knip-only-triage.md)).
2. Achados novos:
   - [ ] Membros de enum sem uso ([34](34-unused-enum-members.md)).
   - [ ] Membros de classe sem uso ([35](35-unused-class-members.md)).
   - [ ] Exports duplicados ([36](36-duplicate-exports.md)).
   - [ ] Binários não declarados ([37](37-unlisted-binaries.md)).
3. Adoção:
   - [ ] GitHub Action ([38](38-github-action.md)).
   - [ ] `prunr init` a partir do Knip ([39](39-init-from-knip.md)).
4. Os itens 15, 21 e 14 de "Depois".

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
