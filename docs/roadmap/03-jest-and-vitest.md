# 3. Jest e Vitest

## Objetivo

Num projeto com Jest ou Vitest, o prunr acha os arquivos de teste e o que a configuração do
runner carrega sem que `entries.test` precise listar nada.

## Escopo

- **Detecção:** o runner está ativo quando o pacote `jest` ou `vitest` está declarado no
  `package.json`.
- **Arquivos de teste:** os padrões da configuração (`testMatch` no Jest, `test.include` no
  Vitest) quando são literais estáticos; senão, os padrões padrão de cada runner. Eles se somam a
  `entries.test`, que continua valendo para o que fica fora.
- **Configuração:** `jest.config.*`, a chave `jest` do `package.json`, `vitest.config.*` e
  `vite.config.*` são arquivos de teste (o que eles importam é usado por teste). Os arquivos de
  `setupFiles`, `setupFilesAfterEnv`, `globalSetup`, `globalTeardown` e `test.setupFiles` também.
- **Pacotes citados como texto:** `preset`, `testEnvironment` (`jsdom` →
  `jest-environment-jsdom`), `transform`, `reporters`, `test.environment` e `snapshotSerializers`
  contam como uso de teste do pacote.
- **Valores dinâmicos:** a configuração é lida estaticamente. O que não é literal fica de fora,
  sem erro, e a documentação diz como cobrir com `entries.test`.

## Critérios de aceite

- [x] Uma fixture Jest (`jest.config.js` com setup e preset) e uma Vitest (`vitest.config.ts`).
- [x] Uma fixture Jest só com os padrões padrão, sem `entries.test`.
- [x] `prunr init` deixa de propor `entries.test` quando o runner já cobre os arquivos.
