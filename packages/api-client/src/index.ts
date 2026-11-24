import createClient from "openapi-fetch";

import type { paths } from "./schema.js";

export type { components, paths } from "./schema.js";

export function createApiClient() {
  return createClient<paths>({ credentials: "same-origin" });
}
