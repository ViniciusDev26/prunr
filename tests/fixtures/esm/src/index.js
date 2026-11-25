import greet, { shout } from "./greet.js";
import * as math from "./math.mjs";
import legacy from "./legacy.cjs";
import { helper } from "./legacy-named.cjs";
import { unusedBinding } from "./math.mjs";
import double from "./bridge.cjs";

console.log(greet(), shout(), math.add(1, 2), legacy.run(), helper(), double(2));

const lazy = await import("./lazy.js");
console.log(lazy);
