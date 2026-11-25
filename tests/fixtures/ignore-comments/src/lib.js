export const used = () => 1;

// prunr-ignore-next-line
export const publicApi = () => 2;

/**
 * Part of the public API, on purpose.
 * @public
 */
export function documented() {}

// prunr-ignore-next-line
export const alsoUsed = 3;

export const dead = 4;

// prunr-ignore-next-line

const nothing = 5;
console.log(nothing);
