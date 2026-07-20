import type { QueryParams } from "../../shared/types";
import type { NewsTagGroup, TagName } from "../types";

/** 官网新闻 worker 的来源配置 */
export type WebCnWorkerConfig<
  Groups extends readonly NewsTagGroup[] = readonly NewsTagGroup[],
> = {
  gameId: string;
  sourceId: "web_cn";
  endpoint: string;
  requestQuery: QueryParams & { iPageSize: number };
  extensionCoverKey: string;
  sourceUrl: (id: string) => string;
  tags: Groups;
  /** 解析规则版本，递增后重算该来源的历史新闻标签 */
  parserVersion: number;
  parser: (title: string, intro: string) => Set<TagName<Groups>>;
};

/** 官网新闻列表接口返回结构 */
export type WebCnNewsResponse = {
  data: {
    iTotal: number;
    list: WebCnNewsItem[];
  };
};

/** 官网新闻列表中的单条原始新闻 */
export type WebCnNewsItem = {
  sTitle: string;
  sContent: string;
  sExt: string;
  dtStartTime: string;
  iInfoId: number;
};
