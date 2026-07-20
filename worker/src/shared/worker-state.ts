import { backendFetch } from "./http";

export type WorkerPhase = "initial_backfill" | "incremental";

export type WorkerSession = {
  workerId: string;
  phase: WorkerPhase;
  checkpoint: unknown;
  runId: string;
  leaseUntil: string;
};

type AcquireWorkerResponse = {
  worker_id: string;
  phase: WorkerPhase;
  status: "running";
  checkpoint: unknown;
  run_id: string;
  lease_until: string;
  last_success_at: string | null;
};

/** 获取 worker 执行权，已被占用时返回 null */
export async function acquireWorker(
  workerType: string,
  sourceId: string | null,
  gameId: string,
): Promise<WorkerSession | null> {
  const response = await postJson("/api/v1/admin/workers/acquire", {
    acquire_id: crypto.randomUUID(),
    worker_type: workerType,
    source_id: sourceId,
    game_id: gameId,
  });

  if (response.status === 409) {
    return null;
  }
  await assertOk(response, "acquire worker");

  const data = (await response.json()) as AcquireWorkerResponse;
  return {
    workerId: data.worker_id,
    phase: data.phase,
    checkpoint: data.checkpoint,
    runId: data.run_id,
    leaseUntil: data.lease_until,
  };
}

/** 保存已完整处理的数据页进度并续租 */
export async function checkpointWorker(
  session: WorkerSession,
  checkpoint: unknown,
): Promise<void> {
  const response = await postJson("/api/v1/admin/workers/checkpoint", {
    worker_id: session.workerId,
    run_id: session.runId,
    checkpoint,
  });
  await assertOk(response, "checkpoint worker");
}

/** 延长当前 worker 执行租约 */
export async function heartbeatWorker(session: WorkerSession): Promise<void> {
  const response = await postJson("/api/v1/admin/workers/heartbeat", {
    worker_id: session.workerId,
    run_id: session.runId,
  });
  await assertOk(response, "heartbeat worker");
}

/** 完成当前 worker 执行批次 */
export async function completeWorker(
  session: WorkerSession,
  phase: WorkerPhase,
  checkpoint: unknown,
): Promise<void> {
  const response = await postJson("/api/v1/admin/workers/complete", {
    worker_id: session.workerId,
    run_id: session.runId,
    phase,
    checkpoint,
  });
  await assertOk(response, "complete worker");
}

/** 记录当前 worker 执行失败 */
export async function failWorker(
  session: WorkerSession,
  error: unknown,
): Promise<void> {
  const response = await postJson("/api/v1/admin/workers/fail", {
    worker_id: session.workerId,
    run_id: session.runId,
    error: errorMessage(error),
  });
  await assertOk(response, "fail worker");
}

function postJson(path: string, body: unknown): Promise<Response> {
  return backendFetch(path, undefined, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

async function assertOk(response: Response, operation: string): Promise<void> {
  if (response.ok) {
    return;
  }

  const responseBody = await response.text();
  throw new Error(
    `Failed to ${operation}: ${response.status} ${responseBody}`,
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
