/** 读取必需的环境变量 */
export function getRequiredEnv(key: string): string {
  const value = process.env[key];
  if (!value) {
    throw new Error(`Missing ${key}`);
  }

  return value;
}
