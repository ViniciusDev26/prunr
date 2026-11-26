import { Cart } from "./cart";
import { Ignored } from "./ignored";

const cart = new Cart();
cart.add(1);
console.log(cart.total, new Ignored());

// Another object's `clear`: without types, it counts for `Cart.clear` too.
const cache = { clear() {} };
cache.clear();

// An entry's classes are its interface: their methods are never reported.
export class Api {
  call() {}
}
