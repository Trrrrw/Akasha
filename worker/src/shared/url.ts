import type { QueryParams } from "./types";

/** 构建带查询参数的 URL，忽略 null 和 undefined */
export function buildUrl(
  input: string,
  base?: string,
  query?: QueryParams,
): URL {
  const url = base ? new URL(input, base) : new URL(input);

  if (!query) {
    return url;
  }

  for (const [key, value] of Object.entries(query)) {
    if (value != null) {
      url.searchParams.set(key, String(value));
    }
  }

  return url;
}
