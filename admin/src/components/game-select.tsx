import { useEffect, useMemo, useState } from "react";

import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

const ALL_GAMES_VALUE = "__all_games__";

export type GameSummary = {
  game_code: string;
  name_en: string;
  name_zh: string;
  index: number;
  cover: string;
  extra: string | null;
  news_count: number;
};

type GamesResponse = {
  games: GameSummary[];
};

export function useGames() {
  const [games, setGames] = useState<GameSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    async function loadGames() {
      setLoading(true);
      setError(null);

      const res = await fetch("/games", {
        signal: controller.signal,
      });

      if (!res.ok) {
        const body = (await res.json().catch(() => null)) as {
          message?: string;
        } | null;
        throw new Error(body?.message ?? "加载游戏列表失败");
      }

      const data = (await res.json()) as GamesResponse;
      setGames(data.games);
    }

    loadGames()
      .catch((err) => {
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err.message : "加载游戏列表失败");
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, []);

  return { games, loading, error };
}

export function GameSelect({
  id,
  label,
  value,
  includeAll = false,
  allLabel = "全部游戏",
  showLabel = true,
  disabled = false,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  includeAll?: boolean;
  allLabel?: string;
  showLabel?: boolean;
  disabled?: boolean;
  onChange: (value: string) => void;
}) {
  const { games, loading, error } = useGames();
  const selectedValue = includeAll && !value ? ALL_GAMES_VALUE : value;
  const placeholder = useMemo(() => {
    if (loading) {
      return "加载中...";
    }

    return "选择游戏";
  }, [loading]);

  return (
    <Field>
      <FieldLabel htmlFor={id} className={showLabel ? undefined : "sr-only"}>
        {label}
      </FieldLabel>
      <Select
        value={selectedValue}
        disabled={disabled || loading}
        onValueChange={(nextValue) =>
          onChange(nextValue === ALL_GAMES_VALUE ? "" : nextValue)
        }
      >
        <SelectTrigger id={id} className="w-full">
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent>
          {includeAll ? (
            <SelectItem value={ALL_GAMES_VALUE}>{allLabel}</SelectItem>
          ) : null}
          {games.map((game) => (
            <SelectItem key={game.game_code} value={game.game_code}>
              {game.name_zh} ({game.game_code})
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      {error ? <FieldError>{error}</FieldError> : null}
    </Field>
  );
}
