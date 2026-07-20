import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const bh3MysWorker = createMysWorker({
  gameId: "bh3",
  sourceId: "mys",
  gids: 1,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("bh3"),
});
