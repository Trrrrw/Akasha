import { Languages } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { languages, type LanguageCode } from "@/data/languages";
import { useLanguage } from "@/lib/language";

export function LanguageSwitcher() {
  const { language, setLanguage } = useLanguage();

  return (
    <Select
      value={language}
      onValueChange={(value) => setLanguage(value as LanguageCode)}
    >
      <SelectTrigger
        size="sm"
        aria-label="切换语言"
        className="size-8 justify-center border-transparent bg-transparent p-0 shadow-none hover:bg-accent hover:text-accent-foreground [&_[data-slot=select-value]]:hidden [&_svg:last-child]:hidden"
      >
        <Languages className="text-inherit" />
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {languages.map((item) => (
            <SelectItem key={item.code} value={item.code}>
              {item.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}
