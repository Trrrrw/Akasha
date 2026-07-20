import { SunMoon, Moon, Sun } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTheme, type Theme } from "@/lib/theme";

const themeIcon = {
  system: SunMoon,
  light: Sun,
  dark: Moon,
} satisfies Record<Theme, typeof SunMoon>;

const themeLabel = {
  system: "根据系统",
  light: "浅色",
  dark: "深色",
} satisfies Record<Theme, string>;

export function ThemeToggle() {
  const { theme, cycleTheme } = useTheme();
  const Icon = themeIcon[theme];

  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      aria-label={`切换外观，当前为${themeLabel[theme]}`}
      title={themeLabel[theme]}
      onClick={cycleTheme}
    >
      <Icon />
    </Button>
  );
}
