import { useGames } from "@/components/game-select";

export default function GamesPage() {
  const { games, loading, error } = useGames();

  return (
    <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border bg-background">
      <div className="grid grid-cols-[96px_160px_minmax(0,1fr)_140px_120px] border-b bg-muted/40 px-4 py-2 text-sm font-medium text-muted-foreground">
        <div>封面</div>
        <div>代码</div>
        <div>名称</div>
        <div>排序</div>
        <div>新闻数</div>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {error ? <div className="p-6 text-sm text-destructive">{error}</div> : null}
        {loading ? (
          <div className="p-6 text-sm text-muted-foreground">正在加载游戏...</div>
        ) : null}
        {!loading && !error && games.length === 0 ? (
          <div className="p-6 text-sm text-muted-foreground">暂无游戏</div>
        ) : null}
        {games.map((game) => (
          <div
            key={game.game_code}
            className="grid grid-cols-[96px_160px_minmax(0,1fr)_140px_120px] items-center border-b px-4 py-3 text-sm"
          >
            <img
              src={game.cover}
              alt=""
              className="h-14 w-20 rounded-md object-cover"
              loading="lazy"
            />
            <div className="text-muted-foreground">{game.game_code}</div>
            <div className="min-w-0 pr-4">
              <div className="truncate font-medium">{game.name_zh}</div>
              <div className="mt-1 truncate text-xs text-muted-foreground">
                {game.name_en}
              </div>
            </div>
            <div className="text-muted-foreground">{game.index}</div>
            <div className="text-muted-foreground">{game.news_count}</div>
          </div>
        ))}
      </div>
    </section>
  );
}
