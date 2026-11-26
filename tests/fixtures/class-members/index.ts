import { Cart } from "./cart";

const cart = new Cart();
cart.add(1);
console.log(cart.total);

// Another object's `clear`: without types, it counts for `Cart.clear` too.
const cache = { clear() {} };
cache.clear();
