import { used, alsoUsed } from "./lib.js";
import { other } from "./legacy.cjs";
import { x } from "./used-file.js";
import { register } from "./plugin-api.js";

console.log(used(), alsoUsed, other(), x, register());

// prunr-ignore-next-line
export const main = () => {};
