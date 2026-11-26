import type { Logger } from "../types/logger.js";
export const log: Logger = () => {};
import svg from "data:image/svg+xml,<svg/>";
import worker from "virtual:worker";
import { serve } from "bun";
console.log(svg, worker, serve);
