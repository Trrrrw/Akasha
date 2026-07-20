import { cn } from "@/lib/utils";
import { usePageToc } from "../page-toc";

const pageTocIndentClass = {
  0: "px-3",
  1: "pr-3 pl-6",
  2: "pr-3 pl-9 text-xs",
  3: "pr-3 pl-12 text-xs",
  4: "pr-3 pl-14 text-xs",
  5: "pr-3 pl-16 text-xs",
} as const;

export function GamePageToc() {
  const { items } = usePageToc();

  if (items.length === 0) {
    return null;
  }

  const firstPageAnchorLevel = Math.min(...items.map((item) => item.level));

  return (
    <aside className="hidden xl:block xl:sticky xl:top-20 xl:self-start">
      <div className="flex flex-col gap-3 text-sm">
        <p className="font-medium">页面目录</p>
        <nav className="flex flex-col gap-1">
          {items.map((item) => {
            const relativeLevel = Math.min(
              item.level - firstPageAnchorLevel,
              5,
            ) as keyof typeof pageTocIndentClass;

            return (
              <a
                key={item.id}
                href={`#${item.id}`}
                className={cn(
                  "block rounded-md py-2 text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground",
                  pageTocIndentClass[relativeLevel],
                )}
              >
                {item.label}
              </a>
            );
          })}
        </nav>
      </div>
    </aside>
  );
}
