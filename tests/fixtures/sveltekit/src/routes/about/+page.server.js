import { env } from "$env/dynamic/private";

export const load = () => ({ key: env.KEY });
