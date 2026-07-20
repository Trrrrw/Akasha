import { getNewsTaggablePage, updateNewsTags } from "./backend";
import { log } from "../shared/logger";
import type { NewsTagDefinition, NewsTagGroup } from "./types";

/** 标签重算使用的新闻内容解析器 */
export type NewsTagParser = (
  title: string,
  intro: string,
) => ReadonlySet<string>;

/** 标签重算的执行参数 */
export type ReclassifyNewsTagsOptions = {
  gameId: string;
  sourceId: string;
  parser: NewsTagParser;
  startOffset: number;
  saveOffset: (nextOffset: number) => Promise<void>;
};

const PAGE_SIZE = 100;

/** 尚未配置规则的来源使用的空标签配置 */
export function createEmptyTagConfig(): {
  tags: readonly [];
  parserVersion: number;
  parser: (title: string, intro: string) => Set<never>;
} {
  return {
    tags: [],
    parserVersion: 1,
    parser: (_title, _intro) => new Set<never>(),
  };
}

/** 将分组标签配置转换为后端同步所需的平铺结构 */
export function flattenTagGroups(
  groups: readonly NewsTagGroup[],
): NewsTagDefinition[] {
  return groups.flatMap((group) =>
    group.tags.map((tag) => ({
      ...tag,
      group: group.group,
      group_index: group.groupIndex,
    })),
  );
}

/** 按公共新闻列表分页重新计算并回写标签 */
export async function reclassifyNewsTags(
  options: ReclassifyNewsTagsOptions,
): Promise<void> {
  let offset = options.startOffset;
  let total = 0;
  let started = false;

  while (true) {
    // 获取当前批次已存新闻
    const page = await getNewsTaggablePage(
      options.gameId,
      options.sourceId,
      PAGE_SIZE,
      offset,
    );

    if (!started) {
      total = page.total;
      started = true;
      log.info(
        "news",
        `reclassifying tags for ${options.gameId}/${options.sourceId}: ${offset}/${total}`,
      );
    }

    if (page.items.length === 0) {
      log.info(
        "news",
        `finished reclassifying tags for ${options.gameId}/${options.sourceId}: ${offset}/${total}`,
      );
      return;
    }

    // 仅用公开标题和简介重新生成标签
    await updateNewsTags({
      game_id: options.gameId,
      source_id: options.sourceId,
      updates: page.items.map((item) => ({
        id: item.id,
        tags: [...options.parser(item.title, item.intro ?? "")],
      })),
    });

    // 每批完成后保存偏移，便于中断恢复
    offset += page.items.length;
    await options.saveOffset(offset);

    log.info(
      "news",
      `reclassified tags for ${options.gameId}/${options.sourceId}: ${offset}/${total}`,
    );

    if (offset >= page.total) {
      log.info(
        "news",
        `finished reclassifying tags for ${options.gameId}/${options.sourceId}: ${offset}/${total}`,
      );
      return;
    }
  }
}
