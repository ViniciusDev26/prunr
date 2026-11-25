/**
 * @param {import("./types").Invoice[]} invoices
 * @param {import('./flags')} [flags]
 * @returns {number}
 */
function total(invoices, flags) {
  return invoices.length + (flags ? 0 : 1);
}

/* import("./not-jsdoc") is only a comment */
// import("./not-jsdoc") neither

module.exports = { total };
