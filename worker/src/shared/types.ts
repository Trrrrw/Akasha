/** URL 查询参数的通用键值类型 */
export type QueryParams = Record<string, string | number | undefined | null>;

/** 单个游戏的同步器 */
export type GameWorker = {
  gameId: string;
  run: () => Promise<void>;
};

/** 后端列表接口的最小响应结构 */
export type ListResponse = {
  total: number;
  items: { id: string }[];
};
