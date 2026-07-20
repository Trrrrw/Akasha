import HSRData from "hsr-data";
import { log } from "../../shared/logger";
import { syncCharacters } from "../backend";
import type { CharacterWorker, SyncCharactersBody } from "../types";

export const srCharWorker: CharacterWorker = {
  gameId: "sr",
  async run() {
    const srData = new HSRData({ language: "cn" });
    const characters = await srData.characters();
    const march7th = await srData.characterbyId("march_7th");
    const evernight = await srData.characterbyId("evernight");
    if (!march7th || !evernight) {
      return;
    }

    const syncBody: SyncCharactersBody = {
      game_id: this.gameId,
      items: [],
    };
    for (const char of characters) {
      if (char._id === march7th._id || char._id === evernight._id) {
        continue;
      }
      if (char.name.includes("开拓者")) {
        continue;
      }

      syncBody.items.push({
        id: char._id.toString(),
        item_id: char.id,
        name: char.name,
        description: char.description,
        gender: undefined,
        birthday_month: undefined,
        birthday_day: undefined,
        cv: char.cv.chinese,
        extra: char,
      });
    }

    for (const character of [march7th, evernight]) {
      syncBody.items.push({
        id: character._id.toString(),
        item_id: character.id,
        name: character.name,
        description: character.description,
        gender: "female",
        birthday_month: 3,
        birthday_day: 7,
        cv: character.cv.chinese,
        extra: character,
      });
    }

    const syncResult = await syncCharacters(syncBody);
    log.info(
      "char",
      `${syncResult.created}/${syncResult.total} characters created, ${syncResult.updated}/${syncResult.total} characters updated`,
    );
  },
};
