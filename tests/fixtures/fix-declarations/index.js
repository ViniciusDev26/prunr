import { used, usedToo } from "./esm.js";
import { kept, also } from "./cjs.cjs";

console.log(used(), usedToo, kept, also);
