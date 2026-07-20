import { backendFetch } from "./http";
import { log } from "./logger";
import type { ListResponse } from "./types";

/** 获取 Akasha 后端中的游戏 ID 列表 */
export async function getGameIds(): Promise<string[]> {
  const response = await backendFetch("/api/v1/games");

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`Failed to get games: ${response.status} ${body}`);
  }

  const data = (await response.json()) as ListResponse;
  const gameIds = data.items.map((game) => game.id);

  log.info("shared", `obtained ${gameIds.length} games`);
  return gameIds;
}
