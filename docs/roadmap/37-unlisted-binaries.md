# 37. Binários não declarados

## Objetivo

Um script do `package.json` que roda `eslint` sem o pacote declarado depende do que estiver
instalado globalmente ou de outro pacote que o traga. O prunr já lê os binários dos scripts para
saber que um pacote é usado; falta o contrário.

## Escopo

- Um achado `missing-binary` (`script: binário`) para o comando de um script que nenhum pacote
  declarado fornece (pelo `bin` do `package.json` instalado, ou pelo nome quando não está
  instalado) e que não é um comando do sistema ou um binário do Node.
- Comandos do sistema conhecidos (`rm`, `cp`, `node`, `npm`, `git`...) nunca são reportados.

## Critérios de aceite

- [ ] Uma fixture com um script que roda um binário declarado, um não declarado e um do sistema.
