import { Link, Outlet } from "react-router";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeToggle } from "@/components/theme-toggle";
import logoUrl from "@/logo.svg";

export function BaseLayout() {
  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <header className="sticky top-0 z-40 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto flex h-14 max-w-screen-2xl items-center justify-between px-4 sm:px-6 lg:px-8">
          <Link to="/" className="flex items-center gap-2 font-semibold">
            <img
              src={logoUrl}
              alt=""
              className="size-8 rounded-md"
              aria-hidden="true"
            />
            <span>Akasha</span>
          </Link>

          <div className="flex items-center gap-1">
            <ThemeToggle />
            <LanguageSwitcher />
          </div>
        </div>
      </header>

      <div className="flex min-h-[calc(100vh-3.5rem)]">
        <main className="mx-auto w-full max-w-screen-2xl px-4 py-8 sm:px-6 lg:px-8">
          <Outlet />
        </main>
      </div>

      <footer className="border-t bg-muted/30">
        <div className="mx-auto flex max-w-screen-2xl flex-col gap-3 px-4 py-6 text-sm text-muted-foreground sm:px-6 md:flex-row md:items-center md:justify-between lg:px-8">
          <p>Akasha</p>
          <p>米哈游游戏资料、官方内容与剧情文本查询服务。</p>
        </div>
      </footer>
    </div>
  );
}

export default BaseLayout;
