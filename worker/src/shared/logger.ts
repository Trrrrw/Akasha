type LogLevel = "debug" | "info" | "warn" | "error";

function writeLog(level: LogLevel, tag: string, message: string): void {
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
