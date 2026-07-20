import {
    BookOpen,
    CalendarDays,
    CircleHelp,
    Flame,
    Newspaper,
    ScrollText,
    Star,
    Swords,
    UserRound,
    type LucideIcon,
} from "lucide-react";

const icons = {
    "book-open": BookOpen,
    "calendar-days": CalendarDays,
    "circle-help": CircleHelp,
    flame: Flame,
    newspaper: Newspaper,
    "scroll-text": ScrollText,
    star: Star,
    swords: Swords,
    "user-round": UserRound,
} satisfies Record<string, LucideIcon>;

export type IconName = keyof typeof icons;

export function resolveIcon(name: string): LucideIcon {
    return icons[name as IconName] ?? CircleHelp;
}
