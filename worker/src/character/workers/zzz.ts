import { Mwn } from "mwn";
import { log } from "../../shared/logger";
import { syncCharacters } from "../backend";
import type {
  CharacterWorker,
  SyncCharactersBody,
  ZzzMwnAskResponse,
} from "../types";

export const zzzCharWorker: CharacterWorker = {
  gameId: "zzz",
  async run() {
    const wikiClient = new Mwn({
      apiUrl: "https://wiki.biligame.com/zzz/api.php",
    });
    const syncBody: SyncCharactersBody = {
      game_id: this.gameId,
      items: [],
    };

    let page = 0;
    const pageSize = 150;
    while (true) {
      const queryString = `[[分类:角色]]|?生日|?实装日期|offset=${page * pageSize}|limit=${pageSize}`;
      const response = (await wikiClient.request({
        action: "ask",
        query: queryString,
      })) as ZzzMwnAskResponse;
      const results = response.query.results;
      if (Array.isArray(results) || Object.keys(results).length === 0) {
        break;
      }

      for (const result of Object.values(results)) {
        let birthday: [number, number] | undefined;
        if (result.printouts.生日) {
          birthday = parseBirthday(result.printouts.生日[0]);
        }
        syncBody.items.push({
          id: Bun.hash(`zzz-${result.fulltext}-${result.fullurl}`).toString(),
          item_id: result.fulltext,
          name: result.fulltext,
          description: undefined,
          gender: undefined,
          birthday_month: birthday?.[0],
          birthday_day: birthday?.[1],
          cv: undefined,
          extra: result,
        });
      }

      page++;
    }

    const syncResult = await syncCharacters(syncBody);
    log.info(
      "char",
      `${syncResult.created}/${syncResult.total} characters created, ${syncResult.updated}/${syncResult.total} characters updated`,
    );
  },
};

function parseBirthday(value?: string): [number, number] | undefined {
  if (!value) {
    return undefined;
  }

  const match = value.match(/^\s*(\d{1,2})\s*月\s*(\d{1,2})\s*日\s*$/);

  if (!match) {
    return undefined;
  }

  const month = Number(match[1]);
  const day = Number(match[2]);

  if (month < 1 || month > 12 || day < 1 || day > 31) {
    return undefined;
  }

  return [month, day];
}
