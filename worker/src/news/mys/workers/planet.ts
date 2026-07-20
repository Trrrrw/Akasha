import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const planetMysWorker = createMysWorker({
  gameId: "planet",
  sourceId: "mys",
  gids: 10,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("planet"),
});
