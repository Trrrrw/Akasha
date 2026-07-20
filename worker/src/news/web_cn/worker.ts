import { externalFetch } from "../../shared/http";
import { log } from "../../shared/logger";
import { syncTags, updateNews } from "../backend";
import type {
  NewsTagGroup,
  NewsWorker,
  NewsWorkerResult,
} from "../types";
import { flattenTagGroups, reclassifyNewsTags } from "../tags";
import {
  cleanContent,
  extractCover,
  extractIntro,
  extractVideoUrl,
  normalizePublishTime,
} from "./content";
import type { WebCnNewsResponse, WebCnWorkerConfig } from "./types";

const MAX_CONSECUTIVE_EXISTING_ITEMS = 10;
const PAGE_DELAY_MS = 500;

/** 官网新闻抓取与标签重算的持久化断点 */
type WebCnCheckpoint = {
  version: 2;
  nextPage: number;
  inProgress: boolean;
  parserVersion?: number;
  tagReclassificationOffset?: number;
};

export function createWebCnWorker<
  const Groups extends readonly NewsTagGroup[],
>(config: WebCnWorkerConfig<Groups>): NewsWorker {
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

      // 恢复上次中断时保存的分页断点
      const savedCheckpoint = parseCheckpoint(context.checkpoint);

      // 解析规则版本变化后按公共新闻列表分页重算已有新闻
      const parserChanged = savedCheckpoint.parserVersion !== config.parserVersion;
      if (parserChanged || savedCheckpoint.tagReclassificationOffset !== undefined) {
        const tagReclassificationOffset = parserChanged
          ? 0
          : (savedCheckpoint.tagReclassificationOffset ?? 0);
        await context.saveCheckpoint(
          createCheckpoint(
            savedCheckpoint.nextPage,
            savedCheckpoint.inProgress,
            config.parserVersion,
            tagReclassificationOffset,
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
                savedCheckpoint.nextPage,
                savedCheckpoint.inProgress,
                config.parserVersion,
                nextOffset,
              ),
            ),
        });
        await context.saveCheckpoint(
          createCheckpoint(
            savedCheckpoint.nextPage,
            savedCheckpoint.inProgress,
            config.parserVersion,
          ),
        );
      }

      // 根据运行阶段选择官网抓取的起始页
      const recoveringIncrementalRun =
        context.phase === "incremental" && savedCheckpoint.inProgress;
      let existingCount = 0;
      let page =
        context.phase === "initial_backfill"
          ? savedCheckpoint.nextPage
          : recoveringIncrementalRun
            ? savedCheckpoint.nextPage
            : 1;
      let totalPages: number | undefined;

      if (context.phase === "incremental" && !recoveringIncrementalRun) {
        // 新一轮增量同步从首页开始
        await context.saveCheckpoint(createCheckpoint(1, true, config.parserVersion));
      }

      // 按页拉取新闻并写入后端
      while (totalPages === undefined || page <= totalPages) {
        const response = await externalFetch(config.endpoint, {
          ...config.requestQuery,
          iPage: page,
        });

        if (!response.ok) {
          const responseBody = await response.text();
          throw new Error(
            `Failed to fetch ${config.gameId}/${config.sourceId}: ${response.status} ${responseBody}`,
          );
        }

        const data = (await response.json()) as WebCnNewsResponse;

        if (totalPages === undefined) {
          totalPages = Math.ceil(data.data.iTotal / config.requestQuery.iPageSize);
        }

        // 空页表示远端列表已经到达末尾
        if (data.data.list.length === 0) {
          return completedCheckpoint(config.parserVersion);
        }

        // 解析并同步当前页中的每条新闻
        for (const item of data.data.list) {
          const videoUrl = extractVideoUrl(item.sContent);
          const intro = videoUrl
            ? extractIntro(item.sContent)
            : cleanContent(item.sContent);

          const result = await updateNews({
            game_id: config.gameId,
            source_id: config.sourceId,
            id: item.iInfoId.toString(),
            title: item.sTitle,
            intro: intro,
            publish_time: normalizePublishTime(item.dtStartTime),
            source_url: config.sourceUrl(item.iInfoId.toString()),
            cover:
              extractCover(
                item.sExt,
                config.extensionCoverKey,
                item.sContent,
              ) || null,
            news_type: videoUrl ? "video" : "article",
            video_url: videoUrl || null,
            tags: [...config.parser(item.sTitle, intro)],
            raw_data: item,
          });

          if (result.status === 201) {
            if (context.phase === "incremental" && !recoveringIncrementalRun) {
              existingCount = 0;
            }
            log.info(
              "news",
              `created ${result.news.title} - ${result.news.id}`,
            );
            continue;
          }

          if (result.status === 200) {
            log.info(
              "news",
              `updated ${result.news.title} - ${result.news.id}`,
            );

            if (context.phase === "incremental" && !recoveringIncrementalRun) {
              existingCount += 1;
              if (existingCount >= MAX_CONSECUTIVE_EXISTING_ITEMS) {
                return completedCheckpoint(config.parserVersion);
              }
            }

            continue;
          }

          throw new Error(`Unexpected status: ${result.status}`);
        }

        // 当前页完成后持久化下一页断点并控制请求间隔
        page += 1;
        await context.saveCheckpoint(
          createCheckpoint(page, true, config.parserVersion),
        );
        if (page <= totalPages) {
          await Bun.sleep(PAGE_DELAY_MS);
        }
      }

      // 全量回填或本轮增量完成后回到增量阶段
      return completedCheckpoint(config.parserVersion);
    },
  };
}

/** 解析官网新闻同步断点 */
function parseCheckpoint(checkpoint: unknown): WebCnCheckpoint {
  if (!isRecord(checkpoint) || checkpoint.version !== 2) {
    return createCheckpoint(1, false);
  }

  const nextPage = checkpoint.nextPage;
  return createCheckpoint(
    typeof nextPage === "number" && Number.isInteger(nextPage) && nextPage > 0
      ? nextPage
      : 1,
    checkpoint.inProgress === true,
    isNonNegativeInteger(checkpoint.parserVersion)
      ? checkpoint.parserVersion
      : undefined,
    isNonNegativeInteger(checkpoint.tagReclassificationOffset)
      ? checkpoint.tagReclassificationOffset
      : undefined,
  );
}

/** 创建官网新闻同步断点 */
function createCheckpoint(
  nextPage: number,
  inProgress: boolean,
  parserVersion?: number,
  tagReclassificationOffset?: number,
): WebCnCheckpoint {
  return {
    version: 2,
    nextPage,
    inProgress,
    ...(parserVersion === undefined ? {} : { parserVersion }),
    ...(tagReclassificationOffset === undefined
      ? {}
      : { tagReclassificationOffset }),
  };
}

/** 创建一次完成后的增量同步状态 */
function completedCheckpoint(parserVersion: number): NewsWorkerResult {
  return {
    phase: "incremental",
    checkpoint: createCheckpoint(1, false, parserVersion),
  };
}

/** 判断值是否为普通对象 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 判断值是否为非负整数 */
function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}
