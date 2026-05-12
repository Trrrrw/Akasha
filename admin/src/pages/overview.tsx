import { Link } from "react-router";
import { useEffect, useMemo, useState } from "react";
import {
  ClockIcon,
  GamepadIcon,
  NewspaperIcon,
  PlaySquareIcon,
  UserRoundIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

type GameSummary = {
  game_code: string;
  name_en: string;
  name_zh: string;
  index: number;
  cover: string;
  extra: string | null;
  news_count: number;
};

type GamesResponse = {
  games: GameSummary[];
};

type NewsItem = {
  remote_id: string;
  game_code: string;
  source: string;
  title: string;
  intro: string | null;
  publish_time: string;
  source_url: string;
  cover: string;
  is_video: boolean;
  video_url: string | null;
  categories: string[];
  tags: string[];
};

type NewsResponse = {
  page: number;
  page_size: number;
  total: number;
  items: NewsItem[];
};

type CharactersResponse = {
  page: number;
  page_size: number;
  total: number;
  characters: unknown[];
};

type NewsMetaResponse = {
  last_crawler_at: string;
  last_published_at: string;
};

type OverviewData = {
  games: GameSummary[];
  latestNews: NewsItem[];
  newsTotal: number;
  videoTotal: number;
  characterTotal: number;
  meta: NewsMetaResponse | null;
};

export default function OverviewPage() {
  const [data, setData] = useState<OverviewData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    async function loadOverview() {
      setLoading(true);
      setError(null);

      const [games, latestNews, allNews, videoNews, characters, meta] =
        await Promise.all([
          fetchJson<GamesResponse>("/games", controller.signal),
          fetchJson<NewsResponse>("/news/items?page=1&page_size=5", controller.signal),
          fetchJson<NewsResponse>("/news/items?page=1&page_size=1", controller.signal),
          fetchJson<NewsResponse>(
            "/news/items?is_video=true&page=1&page_size=1",
            controller.signal,
          ),
          fetchJson<CharactersResponse>(
            "/characters?page=1&page_size=1",
            controller.signal,
          ),
          fetchOptionalJson<NewsMetaResponse>("/news/meta", controller.signal),
        ]);

      setData({
        games: games.games,
        latestNews: latestNews.items,
        newsTotal: allNews.total,
        videoTotal: videoNews.total,
        characterTotal: characters.total,
        meta,
      });
    }

    loadOverview()
      .catch((err) => {
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err.message : "加载概览失败");
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, []);

  const imageNewsTotal = useMemo(() => {
    if (!data) {
      return 0;
    }

    return Math.max(data.newsTotal - data.videoTotal, 0);
  }, [data]);

  return (
    <main className="h-full min-h-0 flex-1 overflow-y-auto overscroll-contain px-4 pb-6">
      <div className="flex flex-col gap-4">
      {error ? <div className="text-sm text-destructive">{error}</div> : null}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          title="游戏"
          value={data?.games.length}
          description="已收录游戏"
          loading={loading}
          icon={<GamepadIcon />}
        />
        <StatCard
          title="新闻"
          value={data?.newsTotal}
          description={`图文 ${imageNewsTotal} 条`}
          loading={loading}
          icon={<NewspaperIcon />}
        />
        <StatCard
          title="视频"
          value={data?.videoTotal}
          description="视频新闻"
          loading={loading}
          icon={<PlaySquareIcon />}
        />
        <StatCard
          title="角色"
          value={data?.characterTotal}
          description="角色资料"
          loading={loading}
          icon={<UserRoundIcon />}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_360px]">
        <Card>
          <CardHeader>
            <CardTitle>最新新闻</CardTitle>
            <CardDescription>按发布时间展示最近 5 条数据</CardDescription>
            <CardAction>
              <Button asChild variant="outline" size="sm">
                <Link to="/news">管理新闻</Link>
              </Button>
            </CardAction>
          </CardHeader>
          <CardContent>
            <div className="flex flex-col">
              {loading ? (
                <div className="py-6 text-sm text-muted-foreground">
                  正在加载新闻...
                </div>
              ) : null}
              {!loading && data?.latestNews.length === 0 ? (
                <div className="py-6 text-sm text-muted-foreground">暂无新闻</div>
              ) : null}
              {data?.latestNews.map((item) => (
                <a
                  key={`${item.source}:${item.game_code}:${item.remote_id}`}
                  href={item.source_url}
                  target="_blank"
                  rel="noreferrer"
                  className="grid grid-cols-[72px_minmax(0,1fr)_120px] items-center gap-4 border-b py-3 text-sm last:border-b-0"
                >
                  <img
                    src={item.cover}
                    alt=""
                    className="h-12 w-16 rounded-md object-cover"
                    loading="lazy"
                  />
                  <div className="min-w-0">
                    <div className="truncate font-medium">{item.title}</div>
                    <div className="mt-1 flex flex-wrap gap-2 text-xs text-muted-foreground">
                      <span>{item.game_code}</span>
                      <span>{item.source}</span>
                      <span>{item.is_video ? "视频" : "图文"}</span>
                    </div>
                  </div>
                  <div className="text-right text-xs text-muted-foreground">
                    {formatDateTime(item.publish_time)}
                  </div>
                </a>
              ))}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>同步状态</CardTitle>
            <CardDescription>新闻爬虫和本地数据时间</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <SyncRow
              label="最近同步"
              value={data?.meta?.last_crawler_at}
              loading={loading}
            />
            <SyncRow
              label="最新新闻"
              value={data?.meta?.last_published_at}
              loading={loading}
            />
            <div className="rounded-lg border p-3">
              <div className="flex items-center gap-2 text-sm font-medium">
                <ClockIcon />
                {syncStatus(data?.meta?.last_crawler_at)}
              </div>
              <div className="mt-1 text-sm text-muted-foreground">
                {syncDistance(data?.meta?.last_crawler_at)}
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>游戏数据</CardTitle>
          <CardDescription>按游戏查看当前新闻收录量</CardDescription>
          <CardAction>
            <Button asChild variant="outline" size="sm">
              <Link to="/games">管理游戏</Link>
            </Button>
          </CardAction>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {data?.games.map((game) => (
              <div
                key={game.game_code}
                className="grid grid-cols-[72px_minmax(0,1fr)_auto] items-center gap-3 rounded-lg border p-3"
              >
                <img
                  src={game.cover}
                  alt=""
                  className="h-12 w-16 rounded-md object-cover"
                  loading="lazy"
                />
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{game.name_zh}</div>
                  <div className="mt-1 truncate text-xs text-muted-foreground">
                    {game.name_en}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-lg font-semibold">{game.news_count}</div>
                  <div className="text-xs text-muted-foreground">新闻</div>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
      </div>
    </main>
  );
}

function StatCard({
  title,
  value,
  description,
  loading,
  icon,
}: {
  title: string;
  value: number | undefined;
  description: string;
  loading: boolean;
  icon: React.ReactNode;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
        <CardAction>{icon}</CardAction>
      </CardHeader>
      <CardContent>
        <div className="text-3xl font-semibold">
          {loading || value === undefined ? "-" : value.toLocaleString("zh-CN")}
        </div>
      </CardContent>
    </Card>
  );
}

function SyncRow({
  label,
  value,
  loading,
}: {
  label: string;
  value: string | undefined;
  loading: boolean;
}) {
  return (
    <div>
      <div className="text-sm text-muted-foreground">{label}</div>
      <div className="mt-1 text-sm font-medium">
        {loading ? "加载中..." : value ? formatDateTime(value) : "暂无数据"}
      </div>
    </div>
  );
}

async function fetchJson<T>(url: string, signal: AbortSignal): Promise<T> {
  const res = await fetch(url, { signal });

  if (!res.ok) {
    const body = (await res.json().catch(() => null)) as { message?: string } | null;
    throw new Error(body?.message ?? "请求失败");
  }

  return (await res.json()) as T;
}

async function fetchOptionalJson<T>(
  url: string,
  signal: AbortSignal,
): Promise<T | null> {
  const res = await fetch(url, { signal });

  if (res.status === 404) {
    return null;
  }

  if (!res.ok) {
    const body = (await res.json().catch(() => null)) as { message?: string } | null;
    throw new Error(body?.message ?? "请求失败");
  }

  return (await res.json()) as T;
}

function formatDateTime(value: string) {
  return new Date(value).toLocaleString("zh-CN", {
    hour12: false,
  });
}

function syncStatus(value: string | undefined) {
  if (!value) {
    return "同步状态未知";
  }

  const hours = (Date.now() - new Date(value).getTime()) / 1000 / 60 / 60;
  return hours > 24 ? "可能需要同步" : "同步正常";
}

function syncDistance(value: string | undefined) {
  if (!value) {
    return "没有最近同步时间";
  }

  const minutes = Math.max(
    Math.floor((Date.now() - new Date(value).getTime()) / 1000 / 60),
    0,
  );

  if (minutes < 60) {
    return `${minutes} 分钟前同步`;
  }

  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return `${hours} 小时前同步`;
  }

  return `${Math.floor(hours / 24)} 天前同步`;
}
