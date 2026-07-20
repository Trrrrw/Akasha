import * as cheerio from "cheerio";

import type { NewsTagGroup, TagName } from "../../types";

/** 原神官网来源的标签定义 */
export const ysWebCnTags = [
  {
    group: "角色",
    groupIndex: 1,
    tags: [
      { name: "角色PV", index: 1 },
      { name: "角色演示", index: 2 },
      { name: "角色预告", index: 3 },
      { name: "拾枝杂谈", index: 4 },
      { name: "角色生日", index: 5 },
      { name: "角色立绘", index: 6 },
      { name: "角色技能演示", index: 7 },
    ],
  },
  {
    group: "PV、动画、短片",
    groupIndex: 2,
    tags: [{ name: "剧情PV", index: 1 }],
  },
  {
    group: "版本",
    groupIndex: 3,
    tags: [
      { name: "纪行", index: 1 },
      { name: "版本更新维护预告", index: 2 },
    ],
  },
  {
    group: "活动",
    groupIndex: 4,
    tags: [
      { name: "游戏活动", index: 1 },
      { name: "祈愿", index: 2 },
      { name: "七圣召唤", index: 3 },
      { name: "网页活动", index: 4 },
    ],
  },
  {
    group: "其他",
    groupIndex: 99,
    tags: [{ name: "表情包", index: 1 }],
  },
] as const satisfies readonly NewsTagGroup[];

type YsTag = TagName<typeof ysWebCnTags>;

/** 根据原神官网新闻标题和正文生成标签 */
export function parseYsWebCnTags(title: string, intro: string): Set<YsTag> {
  const tags = new Set<YsTag>();
  const titleRules: ReadonlyArray<readonly [RegExp, YsTag]> = [
    [/^《原神》角色预告-「[^」]+」$/, "角色预告"],
    [/^《原神》剧情PV-「[^」]+」$/, "剧情PV"],
    [/^.+生日快乐$/, "角色生日"],
    [/^《原神》.+表情包$/, "表情包"],
    [/^「.+纪行」活动说明$/, "纪行"],
    [/^「.+」版本更新维护预告$/, "版本更新维护预告"],
    [/^《原神》.+角色PV——「[^」]+」$/, "角色PV"],
    [/^角色技能演示——.+$/, "角色技能演示"],
  ];

  if (title.includes("祈愿现已开启")) tags.add("祈愿");
  if (/^「七圣召唤」.+：.+$/.test(title)) {
    tags.add("游戏活动").add("七圣召唤");
  } else if (
    !title.includes("祈愿现已开启") &&
    /^「[^」]+」活动(?:[：:].*)?$/.test(title)
  ) {
    tags.add("游戏活动");
  }

  for (const [pattern, tag] of titleRules) {
    if (pattern.test(title)) tags.add(tag);
  }

  if (isCharacterIllustration(title, intro)) tags.add("角色立绘");
  if (title.includes("网页活动")) tags.add("网页活动");

  return tags;
}

/** 判断正文开头是否符合角色立绘的固定格式 */
function isCharacterIllustration(title: string, content: string): boolean {
  const $ = cheerio.load(content);
  const paragraphs = $("p")
    .map((_, element) => $(element).text().trim())
    .get()
    .filter(Boolean);

  return /^▌.+·.+$/.test(paragraphs[0] ?? "") && paragraphs[1] === title;
}
