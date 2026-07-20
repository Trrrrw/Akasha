import { NavLink } from "react-router";
import { useGameModules } from "@/data/game-modules";
import { resolveIcon } from "@/lib/icon-resolver";
import { cn } from "@/lib/utils";

type GameModuleNavProps = {
  basePath: string;
};

export function GameModuleNav({ basePath }: GameModuleNavProps) {
  const { data: gameModules } = useGameModules();

  return (
    <aside className="lg:sticky lg:top-20 lg:self-start">
      <nav className="flex flex-col gap-1">
        {gameModules?.map((item) => {
          const Icon = resolveIcon(item.icon);
          const to = item.href ? `${basePath}/${item.href}` : basePath;

          return (
            <NavLink
              key={item.href || "overview"}
              to={to}
              end={!item.href}
              className={({ isActive }) =>
                cn(
                  "flex h-9 items-center gap-2 rounded-md px-3 text-sm transition-colors",
                  isActive
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                )
              }
            >
              <Icon className="size-4" />
              <span>{item.label}</span>
            </NavLink>
          );
        })}
      </nav>
    </aside>
  );
}
