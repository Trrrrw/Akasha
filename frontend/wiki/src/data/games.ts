import { createCachedData } from "./cache";

/** 游戏列表接口中供 wiki 使用的基础信息 */
export type GameSummary = {
  id: string;
  name: string;
  index: number;
  cover: string | null;
  icon: string | null;
};

/** 游戏列表接口的响应结构 */
type ListGamesResponse = {
  items: GameSummary[];
};

/** 获取后端已收录的游戏列表 */
async function fetchGames(): Promise<GameSummary[]> {
  const response = await fetch("/api/v1/games");

  if (!response.ok) {
    throw new Error(`Failed to fetch games: ${response.status}`);
  }

  const data = (await response.json()) as ListGamesResponse;
  return data.items;
}

const gameResource = createCachedData(fetchGames);

export const getGames = gameResource.getData;
export const useGames = gameResource.useData;
