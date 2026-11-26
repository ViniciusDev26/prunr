function format() {}
function parse() {}

export { format, format as formatDate };

export {
  parse,
  // The old name, kept while callers move to `parse`.
  // prunr-ignore-next-line
  parse as parseDate,
};
