import { getGameIds } from "./backend";
import { log } from "./logger";
import type { GameWorker } from "./types";

/** 依次运行后端已启用游戏对应的同步器 */
export async function runGameWorkers(
  tag: string,
  workers: Partial<Record<string, GameWorker>>,
): Promise<void> {
  const gameIds = await getGameIds();

  for (const gameId of gameIds) {
    const worker = workers[gameId];
    if (!worker) {
      log.warn(tag, `skip ${gameId}: worker not implemented`);
      continue;
    }

    try {
      log.info(tag, `syncing game ${gameId}`);
      await worker.run();
      log.info(tag, `finished game ${gameId}`);
    } catch (error) {
      log.error(tag, `failed to sync game ${gameId}`, error);
    }
  }
}
