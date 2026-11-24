import { useEffect, useState } from "react";
import { createApiClient, type components } from "@toko/api-client";

export const api = createApiClient();
export type Schema = components["schemas"];

export class ApiFailure extends Error {
  constructor(
    message: string,
    readonly status = 0,
    readonly fields: Record<string, string> = {},
  ) {
    super(message);
  }
}

export function readResult<T>(result: {
  data?: T;
  error?: Schema["ApiError"];
  response: Response;
}): T {
  if (result.data !== undefined) return result.data;
  throw new ApiFailure(
    result.error?.error?.message ?? "Permintaan gagal. Coba lagi.",
    result.response.status,
    result.error?.error?.fields,
  );
}

export function failure(error: unknown): ApiFailure {
  return error instanceof ApiFailure ? error : new ApiFailure("Koneksi ke server gagal. Coba lagi.");
}

type Resource<T> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: ApiFailure };

export function useResource<T>(load: (signal: AbortSignal) => Promise<T>) {
  const [refresh, setRefresh] = useState(0);
  const [current, setCurrent] = useState<{
    load: typeof load;
    refresh: number;
    value: Resource<T>;
  }>();

  useEffect(() => {
    const controller = new AbortController();
    load(controller.signal).then(
      (data) => {
        if (!controller.signal.aborted) setCurrent({ load, refresh, value: { status: "ready", data } });
      },
      (error) => {
        if (!controller.signal.aborted) setCurrent({ load, refresh, value: { status: "error", error: failure(error) } });
      },
    );
    return () => controller.abort();
  }, [load, refresh]);

  return {
    state: current?.load === load && current.refresh === refresh ? current.value : { status: "loading" } as const,
    reload: () => setRefresh((value) => value + 1),
  };
}
