const { Money } = require("./money");

function createInvoice(amount) {
  return { amount: Money.format(amount) };
}

function cancelInvoice(id) {
  return { id, cancelled: true };
}

module.exports = { createInvoice, cancelInvoice };
