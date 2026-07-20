import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const srMysWorker = createMysWorker({
  gameId: "sr",
  sourceId: "mys",
  gids: 6,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("sr"),
});
