# 35. Membros de classe sem uso

## Objetivo

Um método ou propriedade pública de uma classe exportada que nenhum arquivo chama é código
morto, mesmo com a classe em uso. O Knip reporta esses membros.

## Escopo

- Um achado `unused-class-member` (`arquivo: Classe.membro`) para o método público de uma classe
  exportada que nenhum arquivo alcançado acessa pelo nome, nem a própria classe.
- Por ser uma análise sem tipos, é conservadora: qualquer acesso a uma propriedade com o mesmo
  nome, em qualquer objeto, conta como uso; membros `private`, `static` de interface conhecida,
  `constructor`, getters e setters de interfaces padrão e membros de classes que estendem uma
  classe de fora do projeto não são reportados.
- `prunr fix` apaga o membro.

## Feito

Os métodos públicos de uma classe exportada que nenhum arquivo de produção chama são
`unused-class-member`. Sem tipos, qualquer leitura de uma propriedade com o mesmo nome conta
como uso. Ficam de fora os métodos de classes que estendem ou implementam outra, os com
decorator, os privados e protegidos, os de nome convencional, os de uma entrada e todos os de um
pacote publicado.

A última regra veio do corpus: sem ela, 444 achados, quase todos API pública de bibliotecas (os
métodos do `DateTime` do luxon, do `Admin` do driver do MongoDB), que a entrada expõe e quem
chama é quem instala. Os 100 projetos do corpus são bibliotecas publicadas, então não têm
achados; o check vale para aplicações.

## Critérios de aceite

- [x] Uma fixture com um método chamado, um sem uso e um com o mesmo nome acessado noutro objeto.
