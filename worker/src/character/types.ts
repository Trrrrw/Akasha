import type { GameWorker } from "../shared/types";

/** 单个游戏的角色同步器 */
export type CharacterWorker = GameWorker;

/** 批量同步角色请求体 */
export type SyncCharactersBody = {
  game_id: string;
  items: SyncCharacterItem[];
};

/** 单个角色同步数据 */
export type SyncCharacterItem = {
  id: string;
  item_id: string;
  name: string;
  description?: string;
  gender?: string;
  birthday_month?: number;
  birthday_day?: number;
  cv?: string;
  extra: unknown;
};

/** 角色同步结果 */
export type SyncCharactersResult = {
  created: number;
  updated: number;
  total: number;
};

/** ZZZ Wiki MWN Ask 接口响应 */
export type ZzzMwnAskResponse = {
  query: {
    results: Record<string, ZzzMwnAskResult> | [];
  };
};

/** ZZZ Wiki MWN Ask 单个角色结果 */
export type ZzzMwnAskResult = {
  printouts: {
    生日?: string[];
    实装日期?: string[];
  };
  fulltext: string;
  fullurl: string;
  namespace: number;
  exists: string;
  displaytitle: string;
};
