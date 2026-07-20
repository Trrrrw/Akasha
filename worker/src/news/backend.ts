import { backendFetch } from "../shared/http";
import { log } from "../shared/logger";
import type { ListResponse } from "../shared/types";
import type {
  NewsTagDefinition,
  NewsItemInfo,
  NewsTaggablePage,
  SyncTagsResult,
  UpdateNewsTagsBody,
  UpdateNewsBody,
  UpdateNewsResult,
} from "./types";

/** 同步新闻标签定义到 Akasha 后端 */
export async function syncTags(
  gameId: string,
  sourceId: string,
  tags: NewsTagDefinition[],
): Promise<SyncTagsResult> {
  const response = await backendFetch(
    "/api/v1/admin/news/tags/sync",
    undefined,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        game_id: gameId,
        source_id: sourceId,
        tags,
      }),
    },
  );

  if (!response.ok) {
    const responseBody = await response.text();
    throw new Error(
      `Failed to sync tags: ${response.status} ${responseBody}`,
    );
  }

  return (await response.json()) as SyncTagsResult;
}

/** 获取供标签重算使用的一页已存新闻 */
export async function getNewsTaggablePage(
  gameId: string,
  sourceId: string,
  limit: number,
  offset: number,
): Promise<NewsTaggablePage> {
  const query = new URLSearchParams({
    source_id: sourceId,
    limit: limit.toString(),
    offset: offset.toString(),
  });
  const response = await backendFetch(
    `/api/v1/games/${gameId}/news?${query}`,
  );

  if (!response.ok) {
    const responseBody = await response.text();
    throw new Error(
      `Failed to list news for tag refresh: ${response.status} ${responseBody}`,
    );
  }

  return (await response.json()) as NewsTaggablePage;
}

/** 批量替换已存新闻的标签 */
export async function updateNewsTags(
  body: UpdateNewsTagsBody,
): Promise<void> {
  const response = await backendFetch(
    "/api/v1/admin/news/tags/update",
    undefined,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    },
  );

  if (response.status === 204) return;

  const responseBody = await response.text();
  throw new Error(
    `Failed to update news tags: ${response.status} ${responseBody}`,
  );
}

/** 写入或更新一条新闻到 Akasha 后端 */
export async function updateNews(
  body: UpdateNewsBody,
): Promise<UpdateNewsResult> {
  const response = await backendFetch(
    "/api/v1/admin/news/update",
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
    throw new Error(`Failed to update news: ${response.status} ${responseBody}`);
  }

  if (response.status !== 200 && response.status !== 201) {
    throw new Error(`Unexpected update news status: ${response.status}`);
  }

  const news = (await response.json()) as NewsItemInfo;

  return {
    status: response.status,
    news,
  };
}

/** 获取 Akasha 后端新闻来源列表 */
export async function getNewsSourceIds(gameId: string): Promise<string[]> {
  const response = await backendFetch(`/api/v1/games/${gameId}/news/sources`);

  if (!response.ok) {
    const responseBody = await response.text();
    throw new Error(
      `Failed to get news sources for ${gameId}: ${response.status} ${responseBody}`,
    );
  }

  const data = (await response.json()) as ListResponse;
  const sourceIds = data.items.map((source) => source.id);

  log.info("news", `obtained ${sourceIds.length} sources for ${gameId}`);
  return sourceIds;
}
