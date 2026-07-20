import { describe, expect, test } from "bun:test";

import { parseYsWebCnTags } from "./ys";

describe("原神官网新闻标签", () => {
  const cases = [
    {
      title: "《原神》角色预告-「桑多涅：致新生」",
      intro: "",
      expected: ["角色预告"],
    },
    {
      title: "「七圣召唤」热斗模式：自行巧局",
      intro: "",
      expected: ["七圣召唤", "游戏活动"],
    },
    {
      title: "普通新闻",
      intro: "",
      expected: [],
    },
    {
      title: "悠悠圣歌，酿风成诗",
      intro:
        '<p>▌颂礼祝祭·塔利雅</p><p>悠悠圣歌，酿风成诗</p><p></p><p>「要化解居民之间的仇怨，当然要申明法令，公平公正，怎么能像塔利雅那样…拉着一对仇家去酒馆，不谈对错，只拼酒量，最后竟然把他们变成了酒友…这算是化解了吗？」</p><p>——琴</p><p></p><p><img src="https://fastcdn.mihoyo.com/content-v2/hk4e/155923/0a5b64bc68caa42da82ba7a6a850f04e_8330804361849519250.jpg" href="" data-origin-width=""></p><p></p><p>很多蒙德人都认为，塔利雅拥有风神的偏爱，每当人们需要寻求风神的指示，他的提问往往能得到风神的回答。</p><p>也正因如此，渴求庇佑的信众们经常围聚在塔利雅跟前，向他诉说烦扰。</p><p>然而这从来没有给塔利雅带去困扰，甚至可以说他乐在其中。他时刻被世间的纷扰吸引着，就如同蒲公英的种子追逐喧嚣的风。</p><p>没有信众叨扰的日子，他往往会主动离开教堂，流连于市井之间，寻找着能让他插一手的麻烦事。</p><p>与其说是布道需要，不如说是兴趣使然。</p><p>「毕竟祷词里有一句就是这样说的啊，听凭风引。」</p><p></p><p><img src="https://fastcdn.mihoyo.com/content-v2/hk4e/155923/2d32d01cb29bbe27820d6b2c64e890b8_4272556059880399671.jpg" href="" data-origin-width=""></p><p></p><p></p><p>关于《原神》</p><p><br>《原神》是由米哈游自研的一款开放世界冒险RPG。你将在游戏中探索一个被称作「提瓦特」的幻想世界。<br>在这广阔的世界中，你可以踏遍七国，邂逅性格各异、能力独特的同伴，与他们一同对抗强敌，踏上寻回血亲之路；也可以不带目的地漫游，沉浸在充满生机的世界里，让好奇心驱使自己发掘各个角落的奥秘……直到你与分离的血亲重聚，在终点见证一切事物的沉淀。《原神》现已登录PS平台、iOS、Android、PC平台，并支持移动端、PC端以及PS平台数据互通，旅行者可自由选择平台和设备开启冒险。</p>',
      expected: ["角色立绘"],
    },
  ] as const;

  for (const item of cases) {
    test(item.title, () => {
      expect([...parseYsWebCnTags(item.title, item.intro)].sort()).toEqual(
        [...item.expected].sort(),
      );
    });
  }
});
