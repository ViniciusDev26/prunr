import { readFileSync } from "node:fs";
import type { Node } from "estree";
import _ from "lodash";
import { format } from "@/format";
import { add } from "./math.js";
import type { Shape } from "./types";
import { Button } from "./ui";

const shape: Shape = { kind: "circle" };
const node: Node | null = null;
console.log(format(add(1, 2)), shape, node, readFileSync, _.chunk, Button);
