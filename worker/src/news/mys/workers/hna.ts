import { createLegacyTagConfig } from "../tags";
import { createMysWorker } from "../worker";

export const hnaMysWorker = createMysWorker({
  gameId: "hna",
  sourceId: "mys",
  gids: 9,
  sourceUrl(gameId, id) {
    return `https://www.miyoushe.com/${gameId}/article/${id}`;
  },
  ...createLegacyTagConfig("hna"),
});
