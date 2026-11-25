/**
 * @typedef {object} Invoice
 * @property {import("./total").Unused} [cycle] types never form a cycle
 */

class Invoice {}
class Draft {}

module.exports = { Invoice, Draft };
