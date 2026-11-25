import { greeting } from "../../../lib/greeting";

export function GET() {
  return Response.json({ greeting: greeting() });
}
