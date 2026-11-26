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

  // prunr-ignore-next-line
  clear() {
    this.items = [];
  }

  // prunr-ignore-next-line
  reset() {}

  legacyTotal() {
    return this.items.length;
  }
}
