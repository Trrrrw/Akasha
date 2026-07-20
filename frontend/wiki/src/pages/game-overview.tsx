import { MarkdownContent } from "@/components/markdown-content";
import { useGameOverview } from "@/data/game-overviews";
import { usePageToc } from "@/layouts/page-toc";
import { useParams } from "react-router";

export default function GameOverviewPage() {
  const { game } = useParams();
  const { setItems } = usePageToc();
  const {
    data: overview,
    error: overviewError,
    isLoading: isOverviewLoading,
  } = useGameOverview(game);
  const markdown = overview?.overview.content_markdown ?? "";

  return (
    <div className="flex flex-col">
      <section className="flex flex-col gap-3">
        {isOverviewLoading && (
          <p className="text-sm text-muted-foreground">正在加载游戏介绍。</p>
        )}
        {overviewError !== null && (
          <p className="text-sm text-destructive">游戏介绍加载失败。</p>
        )}
      </section>

      {overview && (
        <MarkdownContent markdown={markdown} onHeadingsChange={setItems} />
      )}
    </div>
  );
}
