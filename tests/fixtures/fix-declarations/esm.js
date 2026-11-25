/**
 * The header of the file stays.
 */

export function used() {
  return 1;
}

/**
 * Nobody calls this.
 */
export function unusedFunction() {
  return 2;
}

// Nobody builds this.
export class UnusedClass {}

export const UNUSED_CONSTANT = { a: [1, "x"], b: () => 3 };

export const sideEffect = Math.random();

export const shared = 1, alsoShared = 2;

export const usedToo = 5, notUsed = 6;

const local = 4;
export { local };
