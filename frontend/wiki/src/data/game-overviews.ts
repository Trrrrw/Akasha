import { useEffect, useState } from "react";
import { createCachedData } from "./cache";

export type GameOverviewResponse = {
    game: {
        slug: string;
        title: string;
    };
    overview: {
        content_markdown: string;
    };
};

const overviews: GameOverviewResponse[] = [
    {
        game: {
            slug: "gi",
            title: "原神",
        },
        overview: {
            content_markdown: `## 游戏简介

《原神》是由上海米哈游网络科技股份有限公司制作发行的一款开放世界冒险游戏，于2017年1月底立项，原初测试于2019年6月21日开启，再临测试于2020年3月19日开启，启程测试于2020年6月11日开启，PC版技术性开放测试于9月15日开启，公测于2020年9月28日开启。在数据方面，同在官方服务器的情况下，iOS、PC、Android、鸿蒙平台之间的账号数据互通，玩家可以在同一账号下切换设备。2026年1月20日起，《原神》小米服务器正式停服。

游戏发生在一个被称作“提瓦特大陆”的幻想世界，在这里，被神选中的人将被授予“神之眼”，导引元素之力。玩家将扮演一位名为“旅行者”的神秘角色，在自由的旅行中邂逅性格各异、能力独特的同伴们，和他们一起击败强敌，找回失散的亲人——同时，逐步发掘“原神”的真相。

|||
|-|-|
|审批文号|国新出审[2019]2978号|
|ISBN|9787498069054|
|出版单位|华东师范大学电子音像出版社有限公司|
|著作权人|上海米哈游天命科技有限公司|

## 背景设定

这里是七种元素交汇的幻想世界“提瓦特”。

在遥远的过去，人们藉由对神灵的信仰，获赐了驱动元素的力量，得以在荒野中筑起家园。

五百年前，古国的覆灭却使得天地变异……

如今，席卷大陆的灾难已经停息，和平却仍未如期光临。

作为故事的主人公，你从世界之外漂流而来，降临大地。你将在这广阔的世界中，自由旅行、结识同伴、寻找掌控尘世元素的七神，直到与分离的血亲重聚。`,
        },
    },
    {
        game: {
            slug: "hsr",
            title: "崩坏：星穹铁道",
        },
        overview: {
            content_markdown: `## 游戏简介

《崩坏：星穹铁道》围绕星穹列车的开拓旅程展开，将太空科幻、神话意象和回合制队伍战斗结合。玩家会在不同星球和文明之间推进主线与支线故事，并通过队伍构筑、属性克制和弱点击破参与战斗。

## 获奖信息

《崩坏：星穹铁道》获得 The Game Awards 2023 的 Best Mobile Game 奖项，也曾获得 App Store Awards 2023 的 iPhone Game of the Year。`,
        },
    },
    {
        game: {
            slug: "zzz",
            title: "绝区零",
        },
        overview: {
            content_markdown: `## 游戏简介

《绝区零》设定在灾害“空洞”威胁下的新艾利都，玩家以代理人的视角接触不同阵营角色，在高速动作战斗和都市生活叙事中推进故事。游戏突出都市幻想风格、代理人小队切换和节奏鲜明的动作连携。

## 获奖信息

《绝区零》曾获得 New York Game Awards 2025 的 A-Train Award for Best Mobile Game。相关记录后续会随着资料库完善继续补充。`,
        },
    },
];

const overviewResource = createCachedData(async () => overviews);

export async function getGameOverview(slug: string) {
    const data = await overviewResource.getData();

    return data.find((item) => item.game.slug === slug) ?? null;
}

export function useGameOverview(slug: string | undefined) {
    const [state, setState] = useState<{
        data: GameOverviewResponse | null;
        error: unknown;
        isLoading: boolean;
    }>({
        data: null,
        error: null,
        isLoading: slug !== undefined,
    });

    useEffect(() => {
        if (!slug) {
            setState({ data: null, error: null, isLoading: false });
            return;
        }

        let isMounted = true;

        setState((current) => ({
            ...current,
            isLoading: current.data === null,
        }));
        getGameOverview(slug)
            .then((data) => {
                if (isMounted) {
                    setState({ data, error: null, isLoading: false });
                }
            })
            .catch((error) => {
                if (isMounted) {
                    setState({ data: null, error, isLoading: false });
                }
            });

        return () => {
            isMounted = false;
        };
    }, [slug]);

    return state;
}
