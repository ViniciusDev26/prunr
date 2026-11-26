# 35. Membros de classe sem uso

## Objetivo

Um método ou propriedade pública de uma classe exportada que nenhum arquivo chama é código
morto, mesmo com a classe em uso. O Knip reporta esses membros.

## Escopo

- Um achado `unused-class-member` (`arquivo: Classe.membro`) para o membro público de uma classe
  exportada que nenhum arquivo alcançado acessa pelo nome, nem a própria classe.
- Por ser uma análise sem tipos, é conservadora: qualquer acesso a uma propriedade com o mesmo
  nome, em qualquer objeto, conta como uso; membros `private`, `static` de interface conhecida,
  `constructor`, getters e setters de interfaces padrão e membros de classes que estendem uma
  classe de fora do projeto não são reportados.
- `prunr fix` apaga o membro.

## Critérios de aceite

- [ ] Uma fixture com um método chamado, um sem uso e um com o mesmo nome acessado noutro objeto.
