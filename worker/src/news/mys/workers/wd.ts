import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const wdMysWorker = createMysWorker({
  gameId: "wd",
  sourceId: "mys",
  gids: 4,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("wd"),
});
