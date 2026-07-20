import { createEmptyTagConfig } from "../../tags";
import { createWebCnWorker } from "../worker";

export const zzzWebCnWorker = createWebCnWorker({
  gameId: "zzz",
  sourceId: "web_cn",
  endpoint:
    "https://api-takumi-static.mihoyo.com/content_v2_user/app/706fd13a87294881/getContentList",
  requestQuery: {
    iPageSize: 9,
    sLangKey: "zh-cn",
    iChanId: 273,
  },
  extensionCoverKey: "news-banner",
  sourceUrl(id) {
    return `https://zzz.mihoyo.com/news/${id}`;
  },
  ...createEmptyTagConfig(),
});
