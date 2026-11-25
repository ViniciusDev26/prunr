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
- [x] 8. [`require` dinâmico com padrão](08-dynamic-require-patterns.md)
- [x] 11. [Plugins internos](11-plugins.md)

## Depois

- [ ] 7. [Workspaces e monorepo](07-workspaces.md)
- [ ] 9. [Next.js e NestJS](09-next-and-nest.md)
- [ ] 10. [Performance](10-performance.md)
- [ ] 12. [`prunr fix` apaga a declaração que fica sem uso](12-fix-removes-declarations.md)
- [ ] 13. [`prunr fix --test-only`](13-fix-test-only.md)
- [ ] 14. [Ligar e desligar plugins no `prunr.json`](14-toggle-plugins.md)
- [ ] 15. [Pacotes carregados pelo nome por outra lib](15-packages-loaded-by-name.md)

Antes de tudo, e sempre que um item terminar: rodar o prunr no projeto CommonJS de referência e
transformar cada divergência em teste (o teste de aceite do [MVP](../mvp/README.md)).
