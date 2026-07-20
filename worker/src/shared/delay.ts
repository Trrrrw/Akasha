const DEFAULT_JITTER_RATIO = 0.2;

/** 为重试等待加入随机抖动 */
export function jitterDelayMs(
  delayMs: number,
  ratio = DEFAULT_JITTER_RATIO,
): number {
  const factor = 1 - ratio + Math.random() * ratio * 2;
  return Math.max(0, Math.round(delayMs * factor));
}

/** 为服务端指定的最短等待加入仅向后的随机抖动 */
export function jitterMinimumDelayMs(
  delayMs: number,
  ratio = DEFAULT_JITTER_RATIO,
): number {
  const factor = 1 + Math.random() * ratio;
  return Math.max(0, Math.round(delayMs * factor));
}
