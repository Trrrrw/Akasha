import { runGameWorkers } from "../shared/runner";
import type { EventWorker } from "./types";

const workers: Partial<Record<string, EventWorker>> = {};

await runGameWorkers("event", workers);
