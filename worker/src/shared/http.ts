import { getRequiredEnv } from "./config";
import { jitterDelayMs, jitterMinimumDelayMs } from "./delay";
import type { QueryParams } from "./types";
import { buildUrl } from "./url";

export type RetryOptions = {
  maxRetries?: number;
  baseDelayMs?: number;
  maxDelayMs?: number;
};

type RequestInitFactory = RequestInit | (() => RequestInit);

const BACKEND_USER_AGENT = "Akasha-Worker/1.0";
const EXTERNAL_USER_AGENT =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:152.0) Gecko/20100101 Firefox/152.0";

/** 请求 Akasha 后端接口 */
export function backendFetch(
  path: string,
  query?: QueryParams,
  init: RequestInit = {},
  authorization = true,
  retry?: RetryOptions,
): Promise<Response> {
  const headers = new Headers(init.headers);
  headers.set("User-Agent", BACKEND_USER_AGENT);

  if (authorization) {
    headers.set("Authorization", `Bearer ${getRequiredEnv("WORKER_TOKEN")}`);
  }

  return fetchWithRetry(
    buildUrl(path, getRequiredEnv("BACKEND_BASE"), query),
    { ...init, headers },
    retry,
  );
}

/** 请求外部信息源接口 */
export function externalFetch(
  endpoint: string,
  query?: QueryParams,
  init: RequestInitFactory = {},
  retry?: RetryOptions,
): Promise<Response> {
  return fetchWithRetry(
    buildUrl(endpoint, undefined, query),
    () => {
      const requestInit = resolveRequestInit(init);
      const headers = new Headers(requestInit.headers);
      if (!headers.has("User-Agent")) {
        headers.set("User-Agent", EXTERNAL_USER_AGENT);
      }

      return { ...requestInit, headers };
    },
    retry,
  );
}

async function fetchWithRetry(
  input: URL,
  init: RequestInitFactory,
  retry: RetryOptions = {},
): Promise<Response> {
  const maxRetries = retry.maxRetries ?? 10;
  const baseDelayMs = retry.baseDelayMs ?? 1_000;
  const maxDelayMs = retry.maxDelayMs ?? 30_000;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    let retryResponse: Response | undefined;

    try {
      const response = await fetch(input, resolveRequestInit(init));

      if (!shouldRetry(response) || attempt === maxRetries) {
        return response;
      }

      retryResponse = response;
    } catch (error) {
      if (attempt === maxRetries) {
        throw error;
      }
    }

    await Bun.sleep(
      retryDelay(attempt, baseDelayMs, maxDelayMs, retryResponse),
    );
  }

  throw new Error("Fetch retry exhausted");
}

function resolveRequestInit(init: RequestInitFactory): RequestInit {
  return typeof init === "function" ? init() : init;
}

function shouldRetry(response: Response): boolean {
  return response.status === 429 || response.status >= 500;
}

function retryDelay(
  attempt: number,
  baseDelayMs: number,
  maxDelayMs: number,
  response?: Response,
): number {
  const retryAfterMs = parseRetryAfterMs(response);
  if (retryAfterMs !== undefined) {
    return jitterMinimumDelayMs(retryAfterMs);
  }

  return jitterDelayMs(
    Math.min(baseDelayMs * 2 ** attempt, maxDelayMs),
  );
}

function parseRetryAfterMs(response?: Response): number | undefined {
  if (response?.status !== 429) {
    return undefined;
  }

  const value = response.headers.get("Retry-After")?.trim();
  if (!value) {
    return undefined;
  }

  const seconds = Number(value);
  if (Number.isFinite(seconds) && seconds >= 0) {
    return seconds * 1_000;
  }

  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) {
    return undefined;
  }

  return Math.max(0, timestamp - Date.now());
}
