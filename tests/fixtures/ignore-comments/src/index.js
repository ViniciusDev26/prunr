import { used, alsoUsed } from "./lib.js";
import { other } from "./legacy.cjs";
import { x } from "./used-file.js";

console.log(used(), alsoUsed, other(), x);
