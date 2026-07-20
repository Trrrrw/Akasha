import type { WorkerPhase } from "../shared/worker-state";

/** 单个游戏和来源的新闻同步器 */
export type NewsWorker = {
  gameId: string;
  sourceId: string;
  run: (context: NewsWorkerContext) => Promise<NewsWorkerResult>;
};

export type NewsWorkerContext = {
  phase: WorkerPhase;
  checkpoint: unknown;
  saveCheckpoint: (checkpoint: unknown) => Promise<void>;
};

export type NewsWorkerResult = {
  phase: WorkerPhase;
  checkpoint: unknown;
};

/** 新闻标签同步结果 */
export type SyncTagsResult = {
  changed: boolean;
  tags: NewsTagDefinition[];
};

/** 新闻来源的一条标签定义 */
export type NewsTagDefinition = {
  readonly name: string;
  readonly index: number;
  readonly group?: string;
  readonly group_index?: number;
};

/** 新闻标签分组配置 */
export type NewsTagGroup = {
  readonly group?: string;
  readonly groupIndex?: number;
  readonly tags: readonly Pick<
    NewsTagDefinition,
    "name" | "index"
  >[];
};

/** 从标签配置提取全部标签名 */
export type TagName<Groups extends readonly NewsTagGroup[]> =
  Groups[number]["tags"][number]["name"];

/** 写入新闻接口的请求体 */
export type UpdateNewsBody = {
  game_id: string;
  source_id: string;
  id: string;
  title: string;
  intro: string | null;
  publish_time: string;
  source_url: string;
  cover: string | null;
  news_type: string;
  video_url: string | null;
  tags: string[];
  raw_data: unknown;
};

/** 写入新闻接口的返回结果 */
export type UpdateNewsResult = {
  status: 200 | 201;
  news: NewsItemInfo;
};

/** 新闻写入接口返回的信息 */
export type NewsItemInfo = {
  id: string;
  title: string;
  publish_time: string | null;
  source_url: string;
  cover: string | null;
  news_type: string;
  tags: string[];
  video_url: string | null;
  intro: string | null;
};

/** 已存新闻的标签替换请求 */
export type UpdateNewsTagsBody = {
  game_id: string;
  source_id: string;
  updates: {
    id: string;
    tags: string[];
  }[];
};

/** 供标签重算使用的新闻列表项 */
export type NewsTaggableItem = {
  id: string;
  title: string;
  intro: string | null;
};

/** 供标签重算使用的分页新闻列表 */
export type NewsTaggablePage = {
  total: number;
  limit: number;
  offset: number;
  items: NewsTaggableItem[];
};
