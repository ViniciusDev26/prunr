class Invoice {
  static total(items) {
    return items.reduce((sum, item) => sum + item, 0);
  }
  static legacyTotal(items) {
    return Invoice.total(items);
  }
  static unusedHelper() {}
}
module.exports = { Invoice };
