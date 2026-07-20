import * as cheerio from "cheerio";

/** 从官网 sContent HTML 字段中提取纯文本简介 */
export function extractIntro(html: string): string {
  const $ = cheerio.load(html);
  const texts: string[] = [];
  const discards = new Set([
    "关于《原神》",
    "《原神》是由米哈游自研的一款开放世界冒险RPG。你将在游戏中探索一个被称作「提瓦特」的幻想世界。在这广阔的世界中，你可以踏遍七国，邂逅性格各异、能力独特的同伴，与他们一同对抗强敌，踏上寻回血亲之路；也可以不带目的地漫游，沉浸在充满生机的世界里，让好奇心驱使自己发掘各个角落的奥秘……直到你与分离的血亲重聚，在终点见证一切事物的沉淀。《原神》现已登录PS平台、iOS、Android、PC平台，并支持移动端、PC端以及PS平台数据互通，旅行者可自由选择平台和设备开启冒险。",
  ]);

  $("p").each((_, p) => {
    const text = $(p).text().trim();
    if (discards.has(text)) {
      return;
    }

    if (text) {
      texts.push(text);
    } else {
      texts.push("");
    }
  });

  return texts.join("\n").trim();
}

/** 清理正文 HTML 中影响前端展示的内联属性 */
export function cleanContent(html: string): string {
  const $ = cheerio.load(html, null, false);

  $("[style]").removeAttr("style");
  $("[class]").removeAttr("class");
  $("[id]").removeAttr("id");

  return $.html();
}

/** 将官网时间转换为后端需要的 RFC3339 时间 */
export function normalizePublishTime(dtStartTime: string): string {
  const value = dtStartTime.trim();

  if (!value) {
    throw new Error("Missing dtStartTime");
  }

  if (/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(value)) {
    return value.replace(" ", "T") + "+08:00";
  }

  return new Date(value).toISOString();
}

/** 从正文 HTML 中提取视频链接 */
export function extractVideoUrl(html: string): string {
  const selectors = ["p > video", "p > span > video", "div > video"];

  return extractFirstAttribute(html, selectors, "src");
}

/** 从正文 HTML 中提取封面链接 */
export function extractCover(
  extensionJson: string,
  coverKey: string,
  html: string,
): string {
  if (extensionJson && extensionJson !== "{}") {
    try {
      const extension = JSON.parse(extensionJson) as Record<
        string,
        Array<{ url?: string }>
      >;
      const cover = extension[coverKey]?.[0]?.url;

      if (cover) {
        return cover;
      }
    } catch {
      // 扩展字段解析失败时继续从 HTML 中提取
    }
  }

  return extractFirstAttribute(
    html,
    ["div > video", "p > video", "p > span > video", "video"],
    "poster",
  );
}

/** 从 HTML 中按选择器顺序提取第一个属性值 */
function extractFirstAttribute(
  html: string,
  selectors: string[],
  attr: string,
): string {
  const $ = cheerio.load(html, null, false);

  for (const selector of selectors) {
    const value = $(selector).first().attr(attr);

    if (value) {
      return value;
    }
  }

  return "";
}
