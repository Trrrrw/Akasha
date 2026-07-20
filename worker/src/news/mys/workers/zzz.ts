import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const zzzMysWorker = createMysWorker({
  gameId: "zzz",
  sourceId: "mys",
  gids: 8,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("zzz"),
});
