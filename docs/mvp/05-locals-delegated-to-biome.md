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
        "noUnusedImports": "error",
        "noUnusedPrivateClassMembers": "error"
      }
    }
  }
}
```

### Verificação contra o `tsc`

Uma sonda com 10 tipos de símbolo local sem uso (`require` inteiro e desestruturado, `const`,
`let`, função, classe, variável interna, elemento de array desestruturado, campo e método
`#privados`) foi rodada com `tsc` (`checkJs` + `noUnusedLocals`, os códigos que o script antigo
filtrava) e com o Biome:

- O `tsc` acusou os 10.
- O Biome com as três regras acima acusou os mesmos 10, e também um `catch (error)` sem uso.
- Sem `noUnusedPrivateClassMembers`, os dois membros `#privados` escapariam: por isso a regra
  entrou na configuração recomendada.

O projeto de referência está limpo nos dois hoje (0 achados no `tsc`), então a troca não muda o
resultado do gate dele.

## Critérios de aceite

- [x] O README do prunr tem uma seção "O que o prunr não faz" citando o Biome para variáveis locais.
- [x] O prunr não emite nenhum achado sobre variáveis locais (teste com fixture que tem uma
      variável local sem uso → nenhum achado).
- [x] No projeto de referência, a regra do Biome acima cobre o que o `tsc` + `grep` pegava,
      conferido rodando os dois antes de remover o script antigo.
