export class Cart {
  items: number[] = [];

  add(item: number) {
    this.items.push(item);
  }

  get total() {
    return this.sum();
  }

  private sum() {
    return this.items.reduce((a, b) => a + b, 0);
  }

  clear() {
    this.items = [];
  }

  legacyTotal() {
    return this.items.length;
  }
}
