import def, { a, b } from "./lib.js";
import { onlyUnused } from "./lib.js";
import whole, { c } from "./lib.js";
import * as ns from "./other.js";
import "./anon.js";
import "./cjs.cjs";

console.log(def, a, whole, ns.x);
