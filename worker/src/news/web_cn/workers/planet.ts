import { createEmptyTagConfig } from "../../tags";
import { createWebCnWorker } from "../worker";

export const planetWebCnWorker = createWebCnWorker({
  gameId: "planet",
  sourceId: "web_cn",
  endpoint:
    "https://act-api-takumi-static.mihoyo.com/content_v2_user/app/26702175a73c4f67/getContentList",
  requestQuery: {
    isPreview: 0,
    iChanId: 1262,
    iPageSize: 9,
    sLangKey: "zh-cn",
  },
  extensionCoverKey: "720_1",
  sourceUrl(id) {
    return `https://planet.mihoyo.com/news/detail/${id}`;
  },
  ...createEmptyTagConfig(),
});
