import { MarkdownContent } from "@/components/markdown-content";
import { useParams } from "react-router";
import { useGameModules } from "@/data/game-modules";
import { usePageToc } from "@/layouts/page-toc";

export function GameSectionPage() {
  const { module: moduleId } = useParams();
  const { setItems } = usePageToc();
  const { data: gameModules, error, isLoading } = useGameModules();
  const module = gameModules?.find((item) => item.id === moduleId);

  if (isLoading) {
    return <p className="text-sm text-muted-foreground">正在加载模块信息。</p>;
  }

  if (error !== null) {
    return <p className="text-sm text-destructive">模块信息加载失败。</p>;
  }

  if (!module) {
    return <p className="text-sm text-muted-foreground">未找到该资料模块。</p>;
  }

  return (
    <MarkdownContent
      markdown={module.content_markdown}
      onHeadingsChange={setItems}
    />
  );
}
