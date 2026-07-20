import { parseYsWebCnTags, ysWebCnTags } from "../parsers/ys";
import { createWebCnWorker } from "../worker";

export const ysWebCnWorker = createWebCnWorker({
  gameId: "ys",
  sourceId: "web_cn",
  endpoint:
    "https://api-takumi-static.mihoyo.com/content_v2_user/app/16471662a82d418a/getContentList",
  requestQuery: {
    iAppId: 43,
    iChanId: 719,
    iPageSize: 5,
    sLangKey: "zh-cn",
  },
  extensionCoverKey: "720_1",
  sourceUrl(id) {
    return `https://ys.mihoyo.com/main/news/detail/${id}`;
  },
  tags: ysWebCnTags,
  parserVersion: 2,
  parser: parseYsWebCnTags,
});
