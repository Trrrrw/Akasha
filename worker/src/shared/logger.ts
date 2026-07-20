type LogLevel = "debug" | "info" | "warn" | "error";

const LOG_LEVEL_WEIGHT: Record<LogLevel, number> = {
  debug: 10,
  info: 20,
  warn: 30,
  error: 40,
};

/** 读取 worker 的日志输出阈值 */
function getConfiguredLogLevel(): LogLevel {
  const value = process.env.LOG_LEVEL?.trim().toLowerCase();

  if (
    value === "debug" ||
    value === "info" ||
    value === "warn" ||
    value === "error"
  ) {
    return value;
  }

  return "info";
}

const configuredLogLevel = getConfiguredLogLevel();

function writeLog(level: LogLevel, tag: string, message: string): void {
  if (LOG_LEVEL_WEIGHT[level] < LOG_LEVEL_WEIGHT[configuredLogLevel]) {
    return;
  }

  const time =
    new Date()
      .toLocaleString("sv-SE", {
        timeZone: "Asia/Shanghai",
        hour12: false,
      })
      .replace(" ", "T") + "+08:00";

  console[level](`[${time}] [${tag}] ${message}`);
}

export const log = {
  debug: (tag: string, message: string): void => {
    writeLog("debug", tag, message);
  },
  info: (tag: string, message: string): void => {
    writeLog("info", tag, message);
  },
  warn: (tag: string, message: string): void => {
    writeLog("warn", tag, message);
  },
  error: (tag: string, message: string, error?: unknown): void => {
    writeLog("error", tag, message);

    if (error) {
      console.error(error);
    }
  },
};
