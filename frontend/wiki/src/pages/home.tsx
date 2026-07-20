import { GameCard } from "@/components/game-card";
import { useGames } from "@/data/games";

export default function HomePage() {
  const { data: games, error, isLoading } = useGames();

  return (
    <div className="flex flex-col gap-10">
      <section className="flex flex-col gap-4">
        <h1 className="text-4xl font-semibold tracking-normal">Akasha</h1>
        <p className="max-w-2xl text-muted-foreground">
          查询官方内容、角色资料、成就和剧情文本。选择游戏后进入对应资料空间。
        </p>
      </section>

      <section className="grid gap-4 md:grid-cols-3">
        {isLoading && <p className="text-sm text-muted-foreground">正在加载游戏列表。</p>}
        {error !== null && <p className="text-sm text-destructive">游戏列表加载失败。</p>}
        {games?.map((game) => (
          <GameCard
            key={game.slug}
            href={`/games/${game.slug}`}
            image={game.image}
            subtitle={game.subtitle}
            title={game.title}
          />
        ))}
      </section>
    </div>
  );
}
