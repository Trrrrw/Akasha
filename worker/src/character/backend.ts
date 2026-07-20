import { backendFetch } from "../shared/http";
import type { SyncCharactersBody, SyncCharactersResult } from "./types";

/** 同步角色到 Akasha 后端 */
export async function syncCharacters(
  body: SyncCharactersBody,
): Promise<SyncCharactersResult> {
  const response = await backendFetch(
    "/api/v1/admin/chars/sync",
    undefined,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(body),
    },
  );

  if (!response.ok) {
    const responseBody = await response.text();
    throw new Error(
      `Failed to sync characters: ${response.status} ${responseBody}`,
    );
  }

  return (await response.json()) as SyncCharactersResult;
}
