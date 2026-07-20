import { createCachedData } from "./cache";

export type GameSummary = {
  slug: string;
  title: string;
  subtitle: string;
  image: string;
};

const games: GameSummary[] = [
  {
    slug: "gi",
    title: "原神",
    subtitle: "向着星辰与深渊",
    image: "https://uploadstatic.mihoyo.com/contentweb/20190608/2019060812321786065.jpg",
  },
  {
    slug: "hsr",
    title: "崩坏：星穹铁道",
    subtitle: "愿此行，终抵群星。",
    image:
      "https://webstatic.mihoyo.com/upload/op-public/2023/01/24/b74ae5e3a8e8b021b67ea26e27a215f2_184072581688764639.png",
  },
  {
    slug: "zzz",
    title: "绝区零",
    subtitle: "世界全剧终，欢迎来到新艾利都！",
    image:
      "https://webstatic.mihoyo.com/upload/op-public/2022/09/17/a425b5ccb44c72e342cf3a6e488dc445_771169193410538499.jpg",
  },
];

const gameResource = createCachedData(async () => games);

export const getGames = gameResource.getData;
export const useGames = gameResource.useData;
