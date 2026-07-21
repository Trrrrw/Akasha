import { jitterDelayMs } from "../../shared/delay";
import { externalFetch } from "../../shared/http";
import { log } from "../../shared/logger";
import { syncTags, updateNews } from "../backend";
import type {
  NewsTagGroup,
  NewsWorker,
  NewsWorkerContext,
  NewsWorkerResult,
  UpdateNewsResult,
} from "../types";
import { flattenTagGroups, reclassifyNewsTags } from "../tags";
import {
  extractIntro,
  extractVideoUrl,
  normalizePublishTime,
} from "./content";
import {
  createMysRequestInit,
  MYS_DETAIL_ENDPOINT,
  MYS_FIRST_PAGE_ENDPOINT,
  MYS_NEXT_PAGE_ENDPOINT,
} from "./request";
import type {
  MysNewsFull,
  MysNewsItem,
  MysNewsResponse,
  MysWorkerConfig,
} from "./types";

const MIYOUSHE_RETRY_OPTIONS = {
  maxRetries: 10,
  baseDelayMs: 10_000,
  maxDelayMs: 60_000,
};

const LIST_REQUEST_DELAY_MS = 10_000;
/** 米游社详情接口的稳定请求间隔 */
const DETAIL_REQUEST_DELAY_MS = 2_000;
const MISSING_DETAIL_RETRY_DELAY_MS = 10_000;
const MISSING_DETAIL_MAX_RETRIES = 10;
const NEWS_TYPES = [1, 2, 3] as const;

/** 米游社列表接口要求的新闻类型 */
type MysNewsType = (typeof NEWS_TYPES)[number];
/** 各新闻类型已同步到的最新帖子 ID */
type MysBoundaries = Partial<Record<`${MysNewsType}`, string>>;

/** 当前正在翻页的新闻类型断点 */
type MysInProgress = {
  type: MysNewsType;
  nextCursor: string;
  firstPostId: string;
};

/** 米游社新闻抓取与标签重算的持久化断点 */
type MysCheckpoint = {
  version: 1;
  boundaries: MysBoundaries;
  completedTypes?: MysNewsType[];
  parserVersion?: number;
  tagReclassificationOffset?: number;
  inProgress?: MysInProgress;
};

/** 创建按新闻类型分页同步的米游社新闻 worker */
export function createMysWorker<
  const Groups extends readonly NewsTagGroup[],
>(config: MysWorkerConfig<Groups>): NewsWorker {
  return {
    gameId: config.gameId,
    sourceId: config.sourceId,

    async run(context) {
      // 同步来源标签定义
      const result = await syncTags(
        config.gameId,
        config.sourceId,
        flattenTagGroups(config.tags),
      );
      if (result.changed) {
        log.info(
          "news",
          `synced tags for ${config.gameId}/${config.sourceId}`,
        );
      }

      // 恢复每种新闻类型各自的增量边界
      const checkpoint = parseCheckpoint(context.checkpoint);

      // 解析规则版本变化后按公共新闻列表分页重算已有新闻
      const parserChanged = checkpoint.parserVersion !== config.parserVersion;
      if (parserChanged || checkpoint.tagReclassificationOffset !== undefined) {
        const tagReclassificationOffset = parserChanged
          ? 0
          : (checkpoint.tagReclassificationOffset ?? 0);
        await context.saveCheckpoint(
          createCheckpoint(
            checkpoint.boundaries,
            checkpoint.completedTypes,
            config.parserVersion,
            tagReclassificationOffset,
            checkpoint.inProgress,
          ),
        );
        await reclassifyNewsTags({
          gameId: config.gameId,
          sourceId: config.sourceId,
          parser: config.parser,
          startOffset: tagReclassificationOffset,
          saveOffset: (nextOffset) =>
            context.saveCheckpoint(
              createCheckpoint(
                checkpoint.boundaries,
                checkpoint.completedTypes,
                config.parserVersion,
                nextOffset,
                checkpoint.inProgress,
              ),
            ),
        });
        await context.saveCheckpoint(
          createCheckpoint(
            checkpoint.boundaries,
            checkpoint.completedTypes,
            config.parserVersion,
            undefined,
            checkpoint.inProgress,
          ),
        );
      }

      // 复制断点，避免在本轮中直接修改已解析状态
      const boundaries = { ...checkpoint.boundaries };
      const completedTypes = new Set(checkpoint.completedTypes ?? []);
      let inProgress = checkpoint.inProgress;

      // 依次同步米游社要求分别请求的三种新闻类型
      for (const type of NEWS_TYPES) {
        // 当前 type 已经翻完部分页面时，先从该 type 续跑
        if (inProgress && type < inProgress.type) {
          continue;
        }

        if (
          context.phase === "initial_backfill" &&
          completedTypes.has(type)
        ) {
          continue;
        }

        // 拉取当前类型并更新它的最新帖子边界
        const firstPostId = await syncType(
          config,
          type,
          context.phase === "incremental" ? boundaries[type] : undefined,
          inProgress?.type === type ? inProgress : undefined,
          context,
          (nextInProgress) =>
            createCheckpoint(
              boundaries,
              context.phase === "initial_backfill"
                ? [...completedTypes]
                : undefined,
              config.parserVersion,
              undefined,
              nextInProgress,
            ),
        );

        // 当前 type 已完整结束，后续 checkpoint 不再保留其页级断点
        inProgress = undefined;

        if (firstPostId) {
          boundaries[type] = firstPostId;
        }

        if (context.phase === "initial_backfill") {
          completedTypes.add(type);
        }

        await context.saveCheckpoint(
          createCheckpoint(
            boundaries,
            context.phase === "initial_backfill"
              ? [...completedTypes]
              : undefined,
            config.parserVersion,
          ),
        );
      }

      // 所有类型完成后进入后续增量同步阶段
      return completedCheckpoint(boundaries, config.parserVersion);
    },
  };
}

/** 同步米游社的一种新闻类型 */
async function syncType<Groups extends readonly NewsTagGroup[]>(
  config: MysWorkerConfig<Groups>,
  type: MysNewsType,
  boundaryPostId: string | undefined,
  resume: MysInProgress | undefined,
  context: NewsWorkerContext,
  currentCheckpoint: (inProgress: MysInProgress) => MysCheckpoint,
): Promise<string | undefined> {
  let listCursor = resume?.nextCursor ?? "";
  let firstPostId: string | undefined = resume?.firstPostId;

  while (true) {
    // 请求当前 offset 的新闻列表
    const firstPage = listCursor === "";
    const response = await externalFetch(
      firstPage ? MYS_FIRST_PAGE_ENDPOINT : MYS_NEXT_PAGE_ENDPOINT,
      {
        ...(firstPage ? { client_type: 4 } : {}),
        gids: config.gids,
        last_id: listCursor,
        page_size: 20,
        type,
      },
      () => createMysRequestInit({ cookie: !firstPage }),
      MIYOUSHE_RETRY_OPTIONS,
    );

    if (!response.ok) {
      const responseBody = await response.text();
      throw new Error(
        `Failed to fetch ${config.gameId}/${config.sourceId}/type-${type}: ${response.status} ${responseBody}`,
      );
    }

    const data = (await response.json()) as MysNewsResponse;

    // 米游社末页仍可能返回 is_last=false 与递增 cursor，但列表为空
    if (data.data.list.length === 0) {
      log.info(
        "news",
        `mys ${config.gameId}/${config.sourceId}/type-${type} reached an empty page at cursor ${listCursor || "first"}`,
      );
      break;
    }

    // 处理当前页新闻并检查已保存边界
    let reachedBoundary = false;

    for (const item of data.data.list) {
      const postId = item.post.post_id.toString();
      firstPostId ??= postId;

      if (boundaryPostId && postId === boundaryPostId) {
        reachedBoundary = true;
        break;
      }

      const result = await fetchAndUpdatePost(config, item);
      await Bun.sleep(DETAIL_REQUEST_DELAY_MS);
      logUpdateResult(result);
    }

    // 遇到已保存边界、空页或接口末页时结束当前 type
    if (reachedBoundary || data.data.is_last || !data.data.last_id) {
      break;
    }

    listCursor = data.data.last_id;
    await context.saveCheckpoint(
      currentCheckpoint({
        type,
        nextCursor: listCursor,
        firstPostId: firstPostId!,
      }),
    );
    await Bun.sleep(LIST_REQUEST_DELAY_MS);
  }

  return firstPostId;
}

/** 记录一条新闻写入结果 */
function logUpdateResult(result: UpdateNewsResult): void {
  if (result.status === 201) {
    log.info("news", `created ${result.news.title} - ${result.news.id}`);
    return;
  }

  if (result.status === 200) {
    log.info("news", `updated ${result.news.title} - ${result.news.id}`);
    return;
  }

  throw new Error(`Unexpected status: ${result.status}`);
}

/** 解析米游社同步断点 */
function parseCheckpoint(value: unknown): MysCheckpoint {
  if (!isRecord(value) || value.version !== 1) {
    return createCheckpoint({});
  }

  const boundaries = parseBoundaries(value.boundaries);
  const completedTypes = Array.isArray(value.completedTypes)
    ? value.completedTypes.filter(isNewsType)
    : undefined;

  return createCheckpoint(
    boundaries,
    completedTypes,
    isNonNegativeInteger(value.parserVersion) ? value.parserVersion : undefined,
    isNonNegativeInteger(value.tagReclassificationOffset)
      ? value.tagReclassificationOffset
      : undefined,
    parseInProgress(value.inProgress),
  );
}

/** 从未验证数据中提取当前新闻类型的分页断点 */
function parseInProgress(value: unknown): MysInProgress | undefined {
  if (!isRecord(value) || !isNewsType(value.type)) {
    return undefined;
  }

  if (
    typeof value.nextCursor !== "string" ||
    !value.nextCursor ||
    typeof value.firstPostId !== "string" ||
    !value.firstPostId
  ) {
    return undefined;
  }

  return {
    type: value.type,
    nextCursor: value.nextCursor,
    firstPostId: value.firstPostId,
  };
}

/** 从未验证数据中提取各类型的帖子边界 */
function parseBoundaries(value: unknown): MysBoundaries {
  if (!isRecord(value)) {
    return {};
  }

  const boundaries: MysBoundaries = {};
  for (const type of NEWS_TYPES) {
    const boundary = value[type];
    if (typeof boundary === "string" && boundary) {
      boundaries[type] = boundary;
    }
  }
  return boundaries;
}

/** 创建米游社同步断点 */
function createCheckpoint(
  boundaries: MysBoundaries,
  completedTypes?: MysNewsType[],
  parserVersion?: number,
  tagReclassificationOffset?: number,
  inProgress?: MysInProgress,
): MysCheckpoint {
  return {
    version: 1,
    boundaries: { ...boundaries },
    ...(completedTypes ? { completedTypes } : {}),
    ...(parserVersion === undefined ? {} : { parserVersion }),
    ...(tagReclassificationOffset === undefined
      ? {}
      : { tagReclassificationOffset }),
    ...(inProgress === undefined ? {} : { inProgress }),
  };
}

/** 创建一次完成后的增量同步状态 */
function completedCheckpoint(
  boundaries: MysBoundaries,
  parserVersion: number,
): NewsWorkerResult {
  return {
    phase: "incremental",
    checkpoint: createCheckpoint(boundaries, undefined, parserVersion),
  };
}

/** 判断值是否为支持的米游社新闻类型 */
function isNewsType(value: unknown): value is MysNewsType {
  return value === 1 || value === 2 || value === 3;
}

/** 判断值是否为普通对象 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 判断值是否为非负整数 */
function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

/** 获取帖子详情并写入新闻数据 */
async function fetchAndUpdatePost<Groups extends readonly NewsTagGroup[]>(
  config: MysWorkerConfig<Groups>,
  newsItem: MysNewsItem,
): Promise<UpdateNewsResult> {
  let rawData: unknown;
  let data: MysNewsFull | undefined;

  for (let attempt = 0; attempt <= MISSING_DETAIL_MAX_RETRIES; attempt++) {
    const response = await externalFetch(
      MYS_DETAIL_ENDPOINT,
      {
        gids: config.gids,
        post_id: newsItem.post.post_id,
        read: 1,
      },
      () => createMysRequestInit({ cookie: true }),
      MIYOUSHE_RETRY_OPTIONS,
    );

    if (!response.ok) {
      const responseBody = await response.text();
      throw new Error(
        `Failed to fetch ${config.gameId}/${config.sourceId}: ${response.status} ${responseBody}`,
      );
    }

    rawData = await response.json();
    data = rawData as MysNewsFull;

    if (data.data?.post) {
      break;
    }

    const diagnostic = describeMysDetailResponse(rawData);
    if (attempt === MISSING_DETAIL_MAX_RETRIES) {
      throw new Error(
        `Mys detail ${newsItem.post.post_id} returned no post after ${MISSING_DETAIL_MAX_RETRIES} retries: ${diagnostic}`,
      );
    }

    log.warn(
      "news",
      `mys detail ${newsItem.post.post_id} returned no post (${diagnostic}), retrying ${attempt + 1}/${MISSING_DETAIL_MAX_RETRIES}`,
    );
    await Bun.sleep(jitterDelayMs(MISSING_DETAIL_RETRY_DELAY_MS));
  }

  if (!data?.data?.post) {
    throw new Error(`Mys detail ${newsItem.post.post_id} returned no post`);
  }

  const videoUrl = extractVideoUrl(data);
  const intro = extractIntro(data);

  return updateNews({
    game_id: config.gameId,
    source_id: config.sourceId,
    id: newsItem.post.post_id.toString(),
    title: data.data.post.post.subject,
    intro,
    publish_time: normalizePublishTime(data.data.post.post.created_at),
    source_url: config.sourceUrl(
      config.gameId,
      newsItem.post.post_id.toString(),
    ),
    cover: data.data.post.post.cover || null,
    news_type: videoUrl ? "video" : "article",
    video_url: videoUrl || null,
    tags: [...config.parser(data.data.post.post.subject, intro)],
    raw_data: rawData,
  });
}

/** 提取米游社详情失败时可安全输出的响应摘要 */
function describeMysDetailResponse(value: unknown): string {
  if (!isRecord(value)) {
    return `response_type=${Array.isArray(value) ? "array" : typeof value}`;
  }

  const data = isRecord(value.data) ? value.data : undefined;
  const retcode = value.retcode ?? data?.retcode;
  const message = value.message ?? data?.message;
  const dataKeys = data ? Object.keys(data).sort().join(",") : "none";

  return [
    `retcode=${formatDiagnosticValue(retcode)}`,
    `message=${formatDiagnosticValue(message)}`,
    `data_keys=${dataKeys || "empty"}`,
  ].join(" ");
}

/** 将诊断字段限制为可读的标量文本 */
function formatDiagnosticValue(value: unknown): string {
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (value === null) return "null";
  return "missing";
}
