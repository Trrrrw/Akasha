import GenshinData from "genshin-data";
import { log } from "../../shared/logger";
import { syncCharacters } from "../backend";
import type { CharacterWorker, SyncCharactersBody } from "../types";

export const ysCharWorker: CharacterWorker = {
  gameId: "ys",
  async run() {
    const genshinData = new GenshinData({
      language: "chinese-simplified",
    });
    const characters = await genshinData.characters();

    const syncBody: SyncCharactersBody = {
      game_id: this.gameId,
      items: [],
    };
    for (const char of characters) {
      if (char.name.includes("旅行者")) {
        continue;
      }

      syncBody.items.push({
        id: char._id.toString(),
        item_id: char.id,
        name: char.name,
        description: char.description,
        gender: char.gender.id,
        birthday_month: char.birthday[1],
        birthday_day: char.birthday[0],
        cv: char.cv.chinese,
        extra: char,
      });
    }

    syncBody.items.push({
      id: "322001",
      item_id: "paimon",
      name: "派蒙",
      description:
        "「没错！你的幸福就是派蒙的幸福呀！所以反过来说，派蒙幸福，你也幸福。」",
      gender: "female",
      birthday_month: 6,
      birthday_day: 1,
      cv: "多多poi",
      extra: {},
    });

    const syncResult = await syncCharacters(syncBody);
    log.info(
      "char",
      `${syncResult.created}/${syncResult.total} characters created, ${syncResult.updated}/${syncResult.total} characters updated`,
    );
  },
};
