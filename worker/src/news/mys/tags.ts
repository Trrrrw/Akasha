import type { NewsTagGroup } from "../types";

const legacyTagsByGame = {
  bh3: ["游戏PV", "动画短片", "角色PV", "视频集锦", "服装视频", "主题曲/音乐", "过场动画"],
  ys: ["角色演示", "拾枝杂谈", "角色PV", "EP", "MV", "前瞻特别节目", "版本PV", "过场动画", "剧情PV", "角色预告", "PV", "幕后", "动画短片", "提瓦特美食札记", "活动汇总", "衣装PV", "流光拾遗之旅", "蒙德茶会", "璃月雅集", "寻味之旅", "角色逸闻", "CM短片", "景区联动纪录片", "风物集短片", "PV短片"],
  sr: ["走近星穹", "角色PV", "千星纪游", "版本PV", "PV", "动画短片", "EP", "MV", "OP", "黄金史诗PV", "剧情PV", "遥远星球之歌", "前瞻特别节目", "星穹研习会"],
  zzz: ["角色展示", "角色PV", "EP", "MV", "PV", "过场动画", "战斗设计幕后", "代理人战斗情报", "前瞻特别节目", "版本PV", "日常影像", "幕间PV", "动画短片", "策划面对面"],
  planet: ["友邻影像馆", "PV"],
  hna: ["角色PV"],
  wd: [],
} as const;

/** 为暂未细化规则的米游社来源创建临时标签配置 */
export function createLegacyTagConfig<
  const GameId extends keyof typeof legacyTagsByGame,
>(gameId: GameId): {
  tags: readonly [
    {
      readonly tags: readonly {
        readonly name: (typeof legacyTagsByGame)[GameId][number];
        readonly index: number;
      }[];
    },
  ];
  parserVersion: number;
  parser: (
    title: string,
    intro: string,
  ) => Set<(typeof legacyTagsByGame)[GameId][number]>;
} {
  const names = legacyTagsByGame[gameId];
  const tags = [
    {
      tags: names.map((name, index) => ({ name, index })),
    },
  ] as const satisfies readonly NewsTagGroup[];

  return {
    tags,
    parserVersion: 1,
    parser: (title, _intro) =>
      new Set(names.filter((name) => title.includes(name))),
  };
}
