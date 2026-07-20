import { createHash, randomInt } from "node:crypto";

import { getRequiredEnv } from "../../shared/config";

export const MYS_FIRST_PAGE_ENDPOINT =
  "https://bbs-api-static.miyoushe.com/painter/wapi/getNewsList";
export const MYS_NEXT_PAGE_ENDPOINT =
  "https://bbs-api.miyoushe.com/painter/wapi/getNewsList";
export const MYS_DETAIL_ENDPOINT =
  "https://bbs-api.miyoushe.com/post/wapi/getPostFull";

const DEFAULT_APP_VERSION = "2.102.0";
const DEFAULT_DS_SALT = "r3KppdID2yT6ht6P7MxzQykauJj0Cmtg";
const DEFAULT_USER_AGENT =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36 Edg/150.0.0.0";
const DS_RANDOM_CHARACTERS =
  "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

type MysRequestOptions = {
  cookie: boolean;
};

/** 按米游社网页端格式构造请求参数 */
export function createMysRequestInit(
  options: MysRequestOptions,
): RequestInit {
  const cookie = getRequiredEnv("MIYOUSHE_COOKIE");
  const headers = new Headers({
    Accept: "application/json, text/plain, */*",
    "Accept-Language":
      "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6,ja;q=0.5,zh-TW;q=0.4",
    DS: createDs(),
    Origin: "https://www.miyoushe.com",
    Referer: "https://www.miyoushe.com/",
    "User-Agent": process.env.MIYOUSHE_USER_AGENT ?? DEFAULT_USER_AGENT,
    "x-rpc-app_version":
      process.env.MIYOUSHE_APP_VERSION ?? DEFAULT_APP_VERSION,
    "x-rpc-client_type": "4",
    "x-rpc-device_fp": requiredCookieValue(cookie, "DEVICEFP"),
    "x-rpc-device_id": requiredCookieValue(cookie, "_MHYUUID"),
  });

  if (options.cookie) {
    headers.set("Cookie", cookie);
  }

  return { headers };
}

function createDs(): string {
  const timestamp = Math.floor(Date.now() / 1_000);
  const random = Array.from(
    { length: 6 },
    () => DS_RANDOM_CHARACTERS[randomInt(DS_RANDOM_CHARACTERS.length)],
  ).join("");
  const salt = process.env.MIYOUSHE_DS_SALT ?? DEFAULT_DS_SALT;
  const checksum = createHash("md5")
    .update(`salt=${salt}&t=${timestamp}&r=${random}`)
    .digest("hex");

  return `${timestamp},${random},${checksum}`;
}

function requiredCookieValue(cookie: string, name: string): string {
  for (const segment of cookie.split(";")) {
    const separator = segment.indexOf("=");
    if (separator < 0) {
      continue;
    }

    const key = segment.slice(0, separator).trim();
    if (key === name) {
      const value = segment.slice(separator + 1).trim();
      if (value) {
        return value;
      }
    }
  }

  throw new Error(`MIYOUSHE_COOKIE is missing ${name}`);
}
