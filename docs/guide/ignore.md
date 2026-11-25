# Ignorar no código

Para um export que é API pública de propósito (usado por quem instala o pacote, carregado por
nome por um sistema de plugins), marque-o no próprio código:

```js
// prunr-ignore-next-line
export const register = () => {};

/**
 * Parte da API pública.
 * @public
 */
export function createClient() {}

module.exports = {
  // prunr-ignore-next-line: chamado pelo agendador
  hook() {},
};
```

- `// prunr-ignore-next-line` (ou `/* ... */`, com um motivo depois, se quiser) ignora os exports
  feitos na linha seguinte: uma declaração `export`, um `exports.x =`, uma chave de
  `module.exports = { ... }` ou um método estático.
- A tag `@public` num comentário JSDoc faz o mesmo para o export que ele documenta.
- `/* prunr-ignore-file */` em qualquer lugar do arquivo o deixa fora de `unused-file` e de
  `unused-export`.

Um comentário de ignorar que não impede nenhum achado é `unused-ignore`, para não acumular: o
export voltou a ser usado, o comentário não está antes de um export, ou o arquivo ignorado não
tem nada a esconder. A tag `@public` é documentação e nunca é reportada.
