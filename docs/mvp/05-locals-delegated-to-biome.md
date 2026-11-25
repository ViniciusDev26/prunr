# 5. Variáveis locais delegadas ao Biome

## Objetivo

Variáveis, imports e funções locais sem uso são um problema **dentro de um arquivo**, não do
grafo. O Biome já resolve isso. O prunr **não** implementa essa verificação, e isso é uma decisão
de escopo documentada, não uma lacuna.

Isso elimina o truque comum de rodar `tsc` com `noUnusedLocals` num `tsconfig` dedicado e filtrar
os códigos de erro com `grep`.

## Configuração recomendada (Biome 2)

```json
{
  "linter": {
    "rules": {
      "correctness": {
        "noUnusedVariables": "error",
        "noUnusedImports": "error"
      }
    }
  }
}
```

## Critérios de aceite

- [ ] O README do prunr tem uma seção "O que o prunr não faz" citando o Biome para variáveis locais.
- [ ] O prunr não emite nenhum achado sobre variáveis locais (teste com fixture que tem uma
      variável local sem uso → nenhum achado).
- [ ] No projeto de referência, a regra do Biome acima cobre o que o `tsc` + `grep` pegava,
      conferido rodando os dois antes de remover o script antigo.
