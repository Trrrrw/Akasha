import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const ysMysWorker = createMysWorker({
  gameId: "ys",
  sourceId: "mys",
  gids: 2,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("ys"),
});
