/** A constant used only through its export, and nobody uses the export. */
const LIMIT = 100;

const kept = 1;
const also = kept;

function helper() {
  return 1;
}

const computed = Date.now();

module.exports = { LIMIT, kept, also, computed };
exports.helper = helper;
