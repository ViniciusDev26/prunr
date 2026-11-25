const invoices = require("./invoices");

class Money {
  static of(amount) {
    return new Money(amount);
  }
  static format(money) {
    return money.amount.toFixed(2);
  }
  static fromCents(cents) {
    return new Money(cents / 100);
  }
  static invoiceFor(amount) {
    return invoices.createInvoice(Money.of(amount));
  }
  constructor(amount) {
    this.amount = amount;
  }
}

module.exports = { Money };
