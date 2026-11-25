import { readFile, writeFile } from "node:fs/promises";

export function greet() {
  return readFile;
}

export const unusedConst = 1;

// prunr-ignore-next-line
export const stillUsed = 2;

const local = 3;
export { local };
