import { Cart } from "./cart";

export function cart() {
  const cart = new Cart();
  cart.add(1);
  return cart.total;
}
