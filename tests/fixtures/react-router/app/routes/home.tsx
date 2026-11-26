import type { Route } from "./+types/home";
import { greet } from "../lib/greet";

export function loader() {
  return greet();
}

export default function Home({ loaderData }: Route.ComponentProps) {
  return <p>{loaderData}</p>;
}
