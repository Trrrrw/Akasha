import type { NewsTagGroup, TagName } from "../types";

/** 米游社新闻 worker 的来源配置 */
export type MysWorkerConfig<
  Groups extends readonly NewsTagGroup[] = readonly NewsTagGroup[],
> = {
  gameId: string;
  sourceId: "mys";
  gids: number;
  sourceUrl: (gameId: string, id: string) => string;
  tags: Groups;
  /** 解析规则版本，递增后重算该来源的历史新闻标签 */
  parserVersion: number;
  parser: (title: string, intro: string) => Set<TagName<Groups>>;
};

/** 米游社新闻列表接口返回结构 */
export type MysNewsResponse = {
  data: {
    list: MysNewsItem[];
    last_id: string;
    is_last: boolean;
  };
};

/** 米游社新闻列表中的单条原始新闻 */
export type MysNewsItem = {
  post: { post_id: number };
};

/** 米游社新闻详情结构 */
export type MysNewsFull = {
  data: {
    post: {
      post: {
        subject: string;
        content: string;
        cover: string;
        view_type: number;
        created_at: number;
        structured_content: string;
      };
      vod_list: { resolutions: { url: string; bitrate: number }[] }[];
    };
    retcode: number;
    message: string;
  };
};

export type MysStructuredContent = {
  insert: string | MysStructuredImage | MysStructuredVod;
  attributes?: {
    width?: number;
    height?: number;
  };
}[];

export type MysStructuredImage = {
  image: string;
};

export type MysStructuredVod = {
  vod: {
    cover: string;
    resolutions: {
      url: string;
      height: number;
      width: number;
      bitrate: number;
      definition?: string;
      label?: string;
    }[];
  };
};
