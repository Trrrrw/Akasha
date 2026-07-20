import { runGameWorkers } from "../shared/runner";
import type { CharacterWorker } from "./types";
import { srCharWorker } from "./workers/sr";
import { ysCharWorker } from "./workers/ys";
import { zzzCharWorker } from "./workers/zzz";

const workers: Record<string, CharacterWorker> = {
  ys: ysCharWorker,
  sr: srCharWorker,
  zzz: zzzCharWorker,
};

await runGameWorkers("character", workers);
