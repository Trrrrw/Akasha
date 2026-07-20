import { createEmptyTagConfig } from "../../tags";
import { createWebCnWorker } from "../worker";

export const srWebCnWorker = createWebCnWorker({
  gameId: "sr",
  sourceId: "web_cn",
  endpoint:
    "https://api-takumi-static.mihoyo.com/content_v2_user/app/1963de8dc19e461c/getContentList",
  requestQuery: {
    iPageSize: 5,
    sLangKey: "zh-cn",
    isPreview: 0,
    iChanId: 255,
  },
  extensionCoverKey: "news-poster",
  sourceUrl(id) {
    return `https://sr.mihoyo.com/news/${id}`;
  },
  ...createEmptyTagConfig(),
});
