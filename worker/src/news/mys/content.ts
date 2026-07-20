import type { MysNewsFull, MysStructuredContent } from "./types";

/** 从正文数据中提取视频链接 */
export function extractVideoUrl(data: MysNewsFull): string {
  const resolutions = [...(data.data.post.vod_list[0]?.resolutions ?? [])];

  resolutions.sort((a, b) => b.bitrate - a.bitrate);

  return resolutions.find((item) => item.url)?.url ?? "";
}

/** 从米游社结构化正文中提取简介或正文 HTML */
export function extractIntro(data: MysNewsFull): string {
  if (data.data.post.post.view_type === 5) {
    return data.data.post.post.content;
  }

  let structuredContent: MysStructuredContent;
  try {
    structuredContent = JSON.parse(
      data.data.post.post.structured_content,
    ) as MysStructuredContent;
  } catch (error) {
    console.error(error);
    return "";
  }

  const parts: string[] = [];

  for (const node of structuredContent) {
    if (typeof node.insert === "string") {
      if (node.insert) {
        parts.push(`<p>${node.insert}</p>`);
      }
      continue;
    }

    if ("image" in node.insert) {
      parts.push(`<img src="${node.insert.image}">`);
      continue;
    }

    if ("vod" in node.insert) {
      const cover = node.insert.vod.cover;
      const resolutions = [...(node.insert.vod.resolutions ?? [])];
      resolutions.sort((a, b) => b.bitrate - a.bitrate);
      const videoUrl = resolutions.find((item) => item.url)?.url ?? "";
      parts.push(
        `<video controls poster="${cover}"><source src="${videoUrl}" type="video/mp4" /></video>`,
      );
    }
  }

  return parts.join("").trim();
}

/** 将 Unix 秒级时间戳转换为 +08:00 RFC3339 时间 */
export function normalizePublishTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const offsetMs = 8 * 60 * 60 * 1000;
  const local = new Date(date.getTime() + offsetMs);

  return local.toISOString().replace(".000Z", "+08:00");
}
