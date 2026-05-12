import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { BasicDatePicker } from "@/components/basic-date-picker";
import { BirthDatePicker } from "@/components/birth-date-picker";
import { GameSelect } from "@/components/game-select";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldError,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Textarea } from "@/components/ui/textarea";
import { PlusIcon, Trash2Icon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";

const PAGE_SIZE = 20;

type CharacterItem = {
  character_code: string;
  game_code: string;
  name: string;
  birthday_month: number | null;
  birthday_day: number | null;
  release_time: string | null;
  gender: string | null;
  extra: string | null;
};

type CharactersResponse = {
  page: number;
  page_size: number;
  total: number;
  characters: CharacterItem[];
};

type CharacterFilters = {
  name: string;
  game: string;
  gender: "all" | "male" | "female" | "unknown";
};

type CharacterSheetMode = "create" | "edit";

type CharacterFormState = {
  characterCode: string;
  gameCode: string;
  name: string;
  birthdayMonth: string;
  birthdayDay: string;
  releaseTime: string;
  gender: "none" | "male" | "female" | "unknown";
  extra: string;
};

export default function CharactersPage() {
  const [filters, setFilters] = useState<CharacterFilters>({
    name: "",
    game: "",
    gender: "all",
  });
  const [appliedFilters, setAppliedFilters] = useState<CharacterFilters>(filters);
  const [page, setPage] = useState(1);
  const [data, setData] = useState<CharactersResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [sheetOpen, setSheetOpen] = useState(false);
  const [sheetMode, setSheetMode] = useState<CharacterSheetMode>("create");
  const [selectedCharacter, setSelectedCharacter] = useState<CharacterItem | null>(
    null,
  );

  const totalPages = useMemo(() => {
    if (!data || data.total === 0) {
      return 1;
    }

    return Math.ceil(data.total / data.page_size);
  }, [data]);

  useEffect(() => {
    const controller = new AbortController();

    async function loadCharacters() {
      setLoading(true);
      setError(null);

      const params = new URLSearchParams({
        page: String(page),
        page_size: String(PAGE_SIZE),
      });

      appendParam(params, "game", appliedFilters.game);

      const searchName = appliedFilters.name.trim();
      const endpoint = searchName ? "/characters/search" : "/characters";

      if (searchName) {
        params.set("name", searchName);
      } else if (appliedFilters.gender !== "all") {
        params.set("gender", appliedFilters.gender);
      }

      const res = await fetch(`${endpoint}?${params}`, {
        signal: controller.signal,
      });

      if (!res.ok) {
        const body = (await res.json().catch(() => null)) as {
          message?: string;
        } | null;
        throw new Error(body?.message ?? "加载角色失败");
      }

      const nextData = (await res.json()) as CharactersResponse;
      setData(nextData);
    }

    loadCharacters()
      .catch((err) => {
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err.message : "加载角色失败");
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, [appliedFilters, page, reloadKey]);

  function submitFilters(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPage(1);
    setAppliedFilters(filters);
  }

  function resetFilters() {
    const nextFilters: CharacterFilters = {
      name: "",
      game: "",
      gender: "all",
    };

    setFilters(nextFilters);
    setAppliedFilters(nextFilters);
    setPage(1);
  }

  function openCreateSheet() {
    setSheetMode("create");
    setSelectedCharacter(null);
    setSheetOpen(true);
  }

  function openEditSheet(item: CharacterItem) {
    setSheetMode("edit");
    setSelectedCharacter(item);
    setSheetOpen(true);
  }

  function reloadCharacters() {
    setReloadKey((value) => value + 1);
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
      <div className="flex items-start gap-3 rounded-lg border bg-background p-4">
        <form
          className="grid min-w-0 flex-1 gap-3 md:grid-cols-[minmax(0,1fr)_180px_160px_auto_auto]"
          onSubmit={submitFilters}
        >
          <Input
            value={filters.name}
            placeholder="搜索角色名"
            onChange={(event) =>
              setFilters((value) => ({ ...value, name: event.target.value }))
            }
          />
          <GameSelect
            id="character-filter-game"
            label="游戏"
            value={filters.game}
            includeAll
            showLabel={false}
            onChange={(game) =>
              setFilters((value) => ({ ...value, game }))
            }
          />
          <Select
            value={filters.gender}
            onValueChange={(value) =>
              setFilters((current) => ({
                ...current,
                gender: value as CharacterFilters["gender"],
              }))
            }
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="性别" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部性别</SelectItem>
              <SelectItem value="male">男</SelectItem>
              <SelectItem value="female">女</SelectItem>
              <SelectItem value="unknown">未知</SelectItem>
            </SelectContent>
          </Select>
          <Button type="submit">筛选</Button>
          <Button type="button" variant="outline" onClick={resetFilters}>
            重置
          </Button>
        </form>
        <Button type="button" onClick={openCreateSheet}>
          <PlusIcon data-icon="inline-start" />
          新增角色
        </Button>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border bg-background">
        <div className="grid grid-cols-[minmax(0,1fr)_140px_120px_120px_180px] border-b bg-muted/40 px-4 py-2 text-sm font-medium text-muted-foreground">
          <div>角色</div>
          <div>游戏</div>
          <div>性别</div>
          <div>生日</div>
          <div>发布时间</div>
        </div>
        <div className="min-h-0 flex-1 overflow-auto">
          {error ? (
            <div className="p-6 text-sm text-destructive">{error}</div>
          ) : null}
          {loading ? (
            <div className="p-6 text-sm text-muted-foreground">正在加载角色...</div>
          ) : null}
          {!loading && !error && data?.characters.length === 0 ? (
            <div className="p-6 text-sm text-muted-foreground">暂无角色</div>
          ) : null}
          {data?.characters.map((item) => (
            <button
              key={`${item.game_code}:${item.character_code}`}
              type="button"
              className="grid w-full grid-cols-[minmax(0,1fr)_140px_120px_120px_180px] items-center border-b px-4 py-3 text-left text-sm transition-colors hover:bg-muted/50"
              onClick={() => openEditSheet(item)}
            >
              <div className="min-w-0 pr-4">
                <div className="truncate font-medium">{item.name}</div>
                <div className="mt-1 truncate text-xs text-muted-foreground">
                  {item.character_code}
                </div>
              </div>
              <div className="text-muted-foreground">{item.game_code}</div>
              <div className="text-muted-foreground">
                {formatGender(item.gender)}
              </div>
              <div className="text-muted-foreground">
                {formatBirthday(item)}
              </div>
              <div className="text-muted-foreground">
                {item.release_time ? formatDateTime(item.release_time) : "-"}
              </div>
            </button>
          ))}
        </div>
      </div>

      <PaginationBar
        page={page}
        totalPages={totalPages}
        total={data?.total ?? 0}
        loading={loading}
        onPageChange={setPage}
      />

      <CharacterEditorSheet
        mode={sheetMode}
        item={selectedCharacter}
        open={sheetOpen}
        onOpenChange={setSheetOpen}
        onSaved={reloadCharacters}
      />
    </section>
  );
}

function appendParam(params: URLSearchParams, key: string, value: string) {
  const trimmed = value.trim();

  if (trimmed) {
    params.set(key, trimmed);
  }
}

function formatGender(value: string | null) {
  if (value === "male") {
    return "男";
  }

  if (value === "female") {
    return "女";
  }

  return "未知";
}

function formatBirthday(item: CharacterItem) {
  if (!item.birthday_month || !item.birthday_day) {
    return "-";
  }

  return `${item.birthday_month}/${item.birthday_day}`;
}

function formatDateTime(value: string) {
  return new Date(value).toLocaleString("zh-CN", {
    hour12: false,
  });
}

function CharacterEditorSheet({
  mode,
  item,
  open,
  onOpenChange,
  onSaved,
}: {
  mode: CharacterSheetMode;
  item: CharacterItem | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<CharacterFormState>(
    createCharacterForm(item),
  );
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setForm(createCharacterForm(item));
      setError(null);
    }
  }, [item, open]);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(null);

    try {
      const res = await fetch(characterSaveUrl(mode, item), {
        method: mode === "create" ? "POST" : "PUT",
        headers: adminJsonHeaders(),
        body: JSON.stringify(characterPayload(form, mode)),
      });

      if (!res.ok) {
        throw new Error(await responseMessage(res, "保存角色失败"));
      }

      onSaved();
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存角色失败");
    } finally {
      setSaving(false);
    }
  }

  async function deleteCharacter() {
    if (!item) {
      return;
    }

    setDeleting(true);
    setError(null);

    try {
      const res = await fetch(characterItemUrl(item), {
        method: "DELETE",
        headers: adminHeaders(),
      });

      if (!res.ok) {
        throw new Error(await responseMessage(res, "删除角色失败"));
      }

      onSaved();
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除角色失败");
    } finally {
      setDeleting(false);
    }
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-hidden sm:max-w-xl">
        <SheetHeader>
          <SheetTitle>{mode === "create" ? "新增角色" : "编辑角色"}</SheetTitle>
          <SheetDescription>
            修改管理端角色数据，保存后会刷新当前列表。
          </SheetDescription>
        </SheetHeader>

        <form
          id="character-editor-form"
          className="min-h-0 flex-1 overflow-auto px-4"
          onSubmit={submit}
        >
          <FieldGroup className="gap-4">
            <div className="grid gap-4 md:grid-cols-2">
              <Field>
                <FieldLabel htmlFor="character-code">角色代码</FieldLabel>
                <Input
                  id="character-code"
                  value={form.characterCode}
                  disabled={mode === "edit"}
                  onChange={(event) =>
                    setForm((value) => ({
                      ...value,
                      characterCode: event.target.value,
                    }))
                  }
                />
              </Field>
              <GameSelect
                id="character-game-code"
                label="游戏"
                value={form.gameCode}
                disabled={mode === "edit"}
                onChange={(gameCode) =>
                  setForm((value) => ({ ...value, gameCode }))
                }
              />
            </div>

            <Field>
              <FieldLabel htmlFor="character-name">角色名</FieldLabel>
              <Input
                id="character-name"
                value={form.name}
                onChange={(event) =>
                  setForm((value) => ({ ...value, name: event.target.value }))
                }
              />
            </Field>

            <div className="grid gap-4 md:grid-cols-2">
              <BirthDatePicker
                id="character-birthday"
                label="生日"
                month={form.birthdayMonth}
                day={form.birthdayDay}
                onChange={({ month, day }) =>
                  setForm((value) => ({
                    ...value,
                    birthdayMonth: month,
                    birthdayDay: day,
                  }))
                }
              />
              <Field>
                <FieldLabel>性别</FieldLabel>
                <Select
                  value={form.gender}
                  onValueChange={(value) =>
                    setForm((current) => ({
                      ...current,
                      gender: value as CharacterFormState["gender"],
                    }))
                  }
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">未设置</SelectItem>
                    <SelectItem value="male">男</SelectItem>
                    <SelectItem value="female">女</SelectItem>
                    <SelectItem value="unknown">未知</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </div>

            <BasicDatePicker
              id="character-release-time"
              label="发布时间"
              value={form.releaseTime}
              onChange={(releaseTime) =>
                setForm((value) => ({ ...value, releaseTime }))
              }
            />

            <Field>
              <FieldLabel htmlFor="character-extra">扩展信息</FieldLabel>
              <Textarea
                id="character-extra"
                className="min-h-32"
                value={form.extra}
                onChange={(event) =>
                  setForm((value) => ({ ...value, extra: event.target.value }))
                }
              />
            </Field>

            {error ? <FieldError>{error}</FieldError> : null}
          </FieldGroup>
        </form>

        <SheetFooter className="border-t">
          <div className="flex items-center justify-between gap-2">
            {mode === "edit" ? (
              <CharacterDeleteDialog
                disabled={saving || deleting}
                deleting={deleting}
                onConfirm={deleteCharacter}
              />
            ) : (
              <span />
            )}
            <div className="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={saving || deleting}
              >
                取消
              </Button>
              <Button
                type="submit"
                form="character-editor-form"
                disabled={saving || deleting}
              >
                {saving ? "保存中..." : "保存"}
              </Button>
            </div>
          </div>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}

function CharacterDeleteDialog({
  disabled,
  deleting,
  onConfirm,
}: {
  disabled: boolean;
  deleting: boolean;
  onConfirm: () => void;
}) {
  return (
    <AlertDialog>
      <Button type="button" variant="destructive" disabled={disabled} asChild>
        <AlertDialogTrigger>
          <Trash2Icon data-icon="inline-start" />
          删除
        </AlertDialogTrigger>
      </Button>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>删除这个角色？</AlertDialogTitle>
          <AlertDialogDescription>
            删除后管理后台和公共角色列表都会移除这条数据。
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>取消</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            disabled={deleting}
            onClick={onConfirm}
          >
            {deleting ? "删除中..." : "确认删除"}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function createCharacterForm(item: CharacterItem | null): CharacterFormState {
  return {
    characterCode: item?.character_code ?? "",
    gameCode: item?.game_code ?? "",
    name: item?.name ?? "",
    birthdayMonth: item?.birthday_month?.toString() ?? "",
    birthdayDay: item?.birthday_day?.toString() ?? "",
    releaseTime: item?.release_time ?? "",
    gender: (item?.gender as CharacterFormState["gender"]) ?? "none",
    extra: item?.extra ?? "",
  };
}

function characterPayload(form: CharacterFormState, mode: CharacterSheetMode) {
  const payload = {
    name: form.name,
    birthday_month: optionalNumber(form.birthdayMonth),
    birthday_day: optionalNumber(form.birthdayDay),
    release_time: optionalText(form.releaseTime),
    gender: form.gender === "none" ? null : form.gender,
    extra: optionalText(form.extra),
  };

  if (mode === "create") {
    return {
      character_code: form.characterCode,
      game_code: form.gameCode,
      ...payload,
    };
  }

  return payload;
}

function characterSaveUrl(mode: CharacterSheetMode, item: CharacterItem | null) {
  if (mode === "create") {
    return "/admin/data/characters";
  }

  if (!item) {
    return "/admin/data/characters";
  }

  return characterItemUrl(item);
}

function characterItemUrl(item: CharacterItem) {
  return `/admin/data/characters/${encodeURIComponent(
    item.game_code,
  )}/${encodeURIComponent(item.character_code)}`;
}

function optionalNumber(value: string) {
  const trimmed = value.trim();
  return trimmed ? Number(trimmed) : null;
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? value : null;
}

function adminHeaders() {
  const accessToken = localStorage.getItem("access_token");
  return {
    Authorization: `Bearer ${accessToken ?? ""}`,
  };
}

function adminJsonHeaders() {
  return {
    ...adminHeaders(),
    "Content-Type": "application/json",
  };
}

async function responseMessage(res: Response, fallback: string) {
  const body = (await res.json().catch(() => null)) as { message?: string } | null;
  return body?.message ?? fallback;
}

function PaginationBar({
  page,
  totalPages,
  total,
  loading,
  onPageChange,
}: {
  page: number;
  totalPages: number;
  total: number;
  loading: boolean;
  onPageChange: (page: number) => void;
}) {
  return (
    <div className="flex items-center justify-between rounded-lg border bg-background px-4 py-3">
      <div className="text-sm text-muted-foreground">共 {total} 条</div>
      <div className="flex items-center gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={loading || page <= 1}
          onClick={() => onPageChange(page - 1)}
        >
          上一页
        </Button>
        <span className="min-w-24 text-center text-sm text-muted-foreground">
          {page} / {totalPages}
        </span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={loading || page >= totalPages}
          onClick={() => onPageChange(page + 1)}
        >
          下一页
        </Button>
      </div>
    </div>
  );
}
