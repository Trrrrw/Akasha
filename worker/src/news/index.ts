import { getGameIds } from "../shared/backend";
import { log } from "../shared/logger";
import {
  acquireWorker,
  checkpointWorker,
  completeWorker,
  failWorker,
  heartbeatWorker,
} from "../shared/worker-state";
import type { WorkerSession } from "../shared/worker-state";
import { getNewsSourceIds } from "./backend";
import { bh3MysWorker } from "./mys/workers/bh3";
import { hnaMysWorker } from "./mys/workers/hna";
import { planetMysWorker } from "./mys/workers/planet";
import { srMysWorker } from "./mys/workers/sr";
import { wdMysWorker } from "./mys/workers/wd";
import { ysMysWorker } from "./mys/workers/ys";
import { zzzMysWorker } from "./mys/workers/zzz";
import type { NewsWorker } from "./types";
import { planetWebCnWorker } from "./web_cn/workers/planet";
import { srWebCnWorker } from "./web_cn/workers/sr";
import { ysWebCnWorker } from "./web_cn/workers/ys";
import { zzzWebCnWorker } from "./web_cn/workers/zzz";

const workers: Record<string, Record<string, NewsWorker>> = {
  web_cn: {
    ys: ysWebCnWorker,
    sr: srWebCnWorker,
    zzz: zzzWebCnWorker,
    planet: planetWebCnWorker,
  },
  mys: {
    bh3: bh3MysWorker,
    hna: hnaMysWorker,
    planet: planetMysWorker,
    sr: srMysWorker,
    wd: wdMysWorker,
    ys: ysMysWorker,
    zzz: zzzMysWorker,
  },
};

const ACQUIRE_RETRY_DELAY_MS = 10_000;
const ACQUIRE_MAX_ATTEMPTS = 15;
const HEARTBEAT_INTERVAL_MS = 30_000;

async function main() {
  const gameIds = await getGameIds();

  for (const gameId of gameIds) {
    const sourceIds = await getNewsSourceIds(gameId);
    for (const sourceId of sourceIds) {
      const worker = workers[sourceId]?.[gameId];
      if (!worker) {
        log.warn("news", `skip ${gameId}/${sourceId}: worker not implemented`);
        continue;
      }

      await runWorker(worker);
    }
  }
}

async function runWorker(worker: NewsWorker): Promise<void> {
  const label = `${worker.gameId}/${worker.sourceId}`;
  let session: WorkerSession | null;
  try {
    session = await acquireWithRetry(worker, label);
  } catch (error) {
    log.error("news", `failed to acquire ${label}`, error);
    return;
  }

  if (!session) {
    log.warn("news", `skip ${label}: worker is already running`);
    return;
  }

  const stopHeartbeat = startHeartbeat(session, label);
  try {
    log.info("news", `running ${label} in ${session.phase} phase`);
    const result = await worker.run({
      phase: session.phase,
      checkpoint: session.checkpoint,
      saveCheckpoint: (checkpoint) =>
        checkpointWorker(session, checkpoint),
    });
    await stopHeartbeat();
    await completeWorker(session, result.phase, result.checkpoint);
    log.info("news", `finished ${label}`);
  } catch (error) {
    await stopHeartbeat();
    try {
      await failWorker(session, error);
    } catch (stateError) {
      log.error("news", `failed to record ${label} state`, stateError);
    }
    log.error("news", `failed ${label}`, error);
  }
}

async function acquireWithRetry(
  worker: NewsWorker,
  label: string,
): Promise<WorkerSession | null> {
  for (let attempt = 1; attempt <= ACQUIRE_MAX_ATTEMPTS; attempt++) {
    const session = await acquireWorker(
      "news",
      worker.sourceId,
      worker.gameId,
    );
    if (session) {
      return session;
    }

    if (attempt === ACQUIRE_MAX_ATTEMPTS) {
      return null;
    }

    if (attempt === 1) {
      log.warn("news", `waiting for previous ${label} lease to expire`);
    }
    await Bun.sleep(ACQUIRE_RETRY_DELAY_MS);
  }

  return null;
}

function startHeartbeat(
  session: WorkerSession,
  label: string,
): () => Promise<void> {
  let inFlight: Promise<void> | undefined;
  const timer = setInterval(() => {
    if (inFlight) {
      return;
    }

    inFlight = heartbeatWorker(session)
      .catch((error) => {
        log.error("news", `failed to heartbeat ${label}`, error);
      })
      .finally(() => {
        inFlight = undefined;
      });
  }, HEARTBEAT_INTERVAL_MS);

  return async () => {
    clearInterval(timer);
    await inFlight;
  };
}

await main();
