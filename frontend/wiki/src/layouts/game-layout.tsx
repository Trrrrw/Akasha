import { Link } from "react-router";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { cn } from "@/lib/utils";
import { Outlet, useParams } from "react-router";
import { useGameModules } from "@/data/game-modules";
import { useGames } from "@/data/games";
import { PageTocProvider, usePageToc } from "./page-toc";
import { GameModuleNav } from "./game/game-module-nav";
import { GamePageToc } from "./game/game-page-toc";

function GameLayoutContent() {
  const { game, module: moduleId } = useParams();
  const basePath = `/games/${game ?? ""}`;
  const { data: games } = useGames();
  const { data: gameModules } = useGameModules();
  const { items: pageAnchors } = usePageToc();
  const currentGame = games?.find((item) => item.slug === game);
  const currentModule = gameModules?.find((item) => item.id === moduleId);
  const gameTitle = currentGame?.title ?? game;
  const pageTitle = currentModule?.label ?? gameTitle;
  const hasPageAnchors = pageAnchors.length > 0;

  return (
    <div
      className={cn(
        "grid gap-8 lg:grid-cols-[12rem_minmax(0,1fr)]",
        hasPageAnchors && "xl:grid-cols-[12rem_minmax(0,1fr)_12rem]",
      )}
    >
      <GameModuleNav basePath={basePath} />

      <section id="top" className="min-w-0">
        <div className="flex flex-col gap-6">
          <Breadcrumb>
            <BreadcrumbList>
              <BreadcrumbItem>
                <BreadcrumbLink asChild>
                  <Link to="/">首页</Link>
                </BreadcrumbLink>
              </BreadcrumbItem>
              <BreadcrumbSeparator />
              <BreadcrumbItem>
                {currentModule ? (
                  <BreadcrumbLink asChild>
                    <Link to={basePath}>{gameTitle}</Link>
                  </BreadcrumbLink>
                ) : (
                  <BreadcrumbPage>{gameTitle}</BreadcrumbPage>
                )}
              </BreadcrumbItem>
              {currentModule && (
                <>
                  <BreadcrumbSeparator />
                  <BreadcrumbItem>
                    <BreadcrumbPage>{currentModule.label}</BreadcrumbPage>
                  </BreadcrumbItem>
                </>
              )}
            </BreadcrumbList>
          </Breadcrumb>

          <h1 className="scroll-m-20 text-4xl font-extrabold tracking-tight text-balance">
            {pageTitle}
          </h1>

          <Outlet />
        </div>
      </section>

      <GamePageToc />
    </div>
  );
}

export function GameLayout() {
  return (
    <PageTocProvider>
      <GameLayoutContent />
    </PageTocProvider>
  );
}

export default GameLayout;
