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

type NewsItem = {
  remote_id: string;
  game_code: string;
  source: string;
  title: string;
  intro: string | null;
  publish_time: string;
  source_url: string;
  cover: string;
  is_video: boolean;
  video_url: string | null;
  categories: string[];
  tags: string[];
};

type NewsResponse = {
  page: number;
  page_size: number;
  total: number;
  items: NewsItem[];
};

type NewsFilters = {
  game: string;
  source: string;
  category: string;
  tag: string;
  isVideo: "all" | "true" | "false";
};

type NewsSheetMode = "create" | "edit";

type NewsFormState = {
  remoteId: string;
  gameCode: string;
  source: string;
  title: string;
  intro: string;
  publishTime: string;
  sourceUrl: string;
  cover: string;
  isVideo: "false" | "true";
  videoUrl: string;
  categories: string;
  tags: string;
};

export default function NewsPage() {
  const [filters, setFilters] = useState<NewsFilters>({
    game: "",
    source: "",
    category: "",
    tag: "",
    isVideo: "all",
  });
  const [appliedFilters, setAppliedFilters] = useState<NewsFilters>(filters);
  const [page, setPage] = useState(1);
  const [data, setData] = useState<NewsResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState(0);
  const [sheetOpen, setSheetOpen] = useState(false);
  const [sheetMode, setSheetMode] = useState<NewsSheetMode>("create");
  const [selectedNews, setSelectedNews] = useState<NewsItem | null>(null);

  const totalPages = useMemo(() => {
    if (!data || data.total === 0) {
      return 1;
    }

    return Math.ceil(data.total / data.page_size);
  }, [data]);

  useEffect(() => {
    const controller = new AbortController();

    async function loadNews() {
      setLoading(true);
      setError(null);

      const params = new URLSearchParams({
        page: String(page),
        page_size: String(PAGE_SIZE),
      });

      appendParam(params, "game", appliedFilters.game);
      appendParam(params, "source", appliedFilters.source);
      appendParam(params, "category", appliedFilters.category);
      appendParam(params, "tag", appliedFilters.tag);

      if (appliedFilters.isVideo !== "all") {
        params.set("is_video", appliedFilters.isVideo);
      }

      const res = await fetch(`/news/items?${params}`, {
        signal: controller.signal,
      });

      if (!res.ok) {
        const body = (await res.json().catch(() => null)) as {
          message?: string;
        } | null;
        throw new Error(body?.message ?? "加载新闻失败");
      }

      const nextData = (await res.json()) as NewsResponse;
      setData(nextData);
    }

    loadNews()
      .catch((err) => {
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err.message : "加载新闻失败");
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
    const nextFilters: NewsFilters = {
      game: "",
      source: "",
      category: "",
      tag: "",
      isVideo: "all",
    };

    setFilters(nextFilters);
    setAppliedFilters(nextFilters);
    setPage(1);
  }

  function openCreateSheet() {
    setSheetMode("create");
    setSelectedNews(null);
    setSheetOpen(true);
  }

  function openEditSheet(item: NewsItem) {
    setSheetMode("edit");
    setSelectedNews(item);
    setSheetOpen(true);
  }

  function reloadNews() {
    setReloadKey((value) => value + 1);
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden">
      <div className="flex items-start gap-3 rounded-lg border bg-background p-4">
        <form
          className="grid min-w-0 flex-1 gap-3 md:grid-cols-[1fr_1fr_1fr_1fr_160px_auto_auto]"
          onSubmit={submitFilters}
        >
          <GameSelect
            id="news-filter-game"
            label="游戏"
            value={filters.game}
            includeAll
            showLabel={false}
            onChange={(game) =>
              setFilters((value) => ({ ...value, game }))
            }
          />
          <Input
            value={filters.source}
            placeholder="来源"
            onChange={(event) =>
              setFilters((value) => ({ ...value, source: event.target.value }))
            }
          />
          <Input
            value={filters.category}
            placeholder="分类"
            onChange={(event) =>
              setFilters((value) => ({
                ...value,
                category: event.target.value,
              }))
            }
          />
          <Input
            value={filters.tag}
            placeholder="标签"
            onChange={(event) =>
              setFilters((value) => ({ ...value, tag: event.target.value }))
            }
          />
          <Select
            value={filters.isVideo}
            onValueChange={(value) =>
              setFilters((current) => ({
                ...current,
                isVideo: value as NewsFilters["isVideo"],
              }))
            }
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder="类型" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部类型</SelectItem>
              <SelectItem value="false">图文</SelectItem>
              <SelectItem value="true">视频</SelectItem>
            </SelectContent>
          </Select>
          <Button type="submit">筛选</Button>
          <Button type="button" variant="outline" onClick={resetFilters}>
            重置
          </Button>
        </form>
        <Button type="button" onClick={openCreateSheet}>
          <PlusIcon data-icon="inline-start" />
          新增新闻
        </Button>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border bg-background">
        <div className="grid grid-cols-[96px_minmax(0,1fr)_120px_120px_160px] border-b bg-muted/40 px-4 py-2 text-sm font-medium text-muted-foreground">
          <div>封面</div>
          <div>标题</div>
          <div>游戏</div>
          <div>来源</div>
          <div>发布时间</div>
        </div>
        <div className="min-h-0 flex-1 overflow-auto">
          {error ? (
            <div className="p-6 text-sm text-destructive">{error}</div>
          ) : null}
          {loading ? (
            <div className="p-6 text-sm text-muted-foreground">正在加载新闻...</div>
          ) : null}
          {!loading && !error && data?.items.length === 0 ? (
            <div className="p-6 text-sm text-muted-foreground">暂无新闻</div>
          ) : null}
          {data?.items.map((item) => (
            <button
              key={`${item.source}:${item.game_code}:${item.remote_id}`}
              type="button"
              className="grid w-full grid-cols-[96px_minmax(0,1fr)_120px_120px_160px] items-center gap-0 border-b px-4 py-3 text-left text-sm transition-colors hover:bg-muted/50"
              onClick={() => openEditSheet(item)}
            >
              <img
                src={item.cover}
                alt=""
                className="h-14 w-20 rounded-md object-cover"
                loading="lazy"
              />
              <div className="min-w-0 pr-4">
                <div className="truncate font-medium">{item.title}</div>
                <div className="mt-1 flex flex-wrap gap-1 text-xs text-muted-foreground">
                  {item.is_video ? <span>视频</span> : <span>图文</span>}
                  {item.categories.map((category) => (
                    <span key={category}>{category}</span>
                  ))}
                </div>
              </div>
              <div className="text-muted-foreground">{item.game_code}</div>
              <div className="text-muted-foreground">{item.source}</div>
              <div className="text-muted-foreground">
                {formatDateTime(item.publish_time)}
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

      <NewsEditorSheet
        mode={sheetMode}
        item={selectedNews}
        open={sheetOpen}
        onOpenChange={setSheetOpen}
        onSaved={reloadNews}
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

function formatDateTime(value: string) {
  return new Date(value).toLocaleString("zh-CN", {
    hour12: false,
  });
}

function NewsEditorSheet({
  mode,
  item,
  open,
  onOpenChange,
  onSaved,
}: {
  mode: NewsSheetMode;
  item: NewsItem | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<NewsFormState>(createNewsForm(item));
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setForm(createNewsForm(item));
      setError(null);
    }
  }, [item, open]);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError(null);

    try {
      const res = await fetch(newsSaveUrl(mode, item), {
        method: mode === "create" ? "POST" : "PUT",
        headers: adminJsonHeaders(),
        body: JSON.stringify(newsPayload(form, mode)),
      });

      if (!res.ok) {
        throw new Error(await responseMessage(res, "保存新闻失败"));
      }

      onSaved();
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "保存新闻失败");
    } finally {
      setSaving(false);
    }
  }

  async function deleteNews() {
    if (!item) {
      return;
    }

    setDeleting(true);
    setError(null);

    try {
      const res = await fetch(newsItemUrl(item), {
        method: "DELETE",
        headers: adminHeaders(),
      });

      if (!res.ok) {
        throw new Error(await responseMessage(res, "删除新闻失败"));
      }

      onSaved();
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "删除新闻失败");
    } finally {
      setDeleting(false);
    }
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full overflow-hidden sm:max-w-xl">
        <SheetHeader>
          <SheetTitle>{mode === "create" ? "新增新闻" : "编辑新闻"}</SheetTitle>
          <SheetDescription>
            修改管理端新闻数据，保存后会刷新当前列表。
          </SheetDescription>
        </SheetHeader>

        <form
          id="news-editor-form"
          className="min-h-0 flex-1 overflow-auto px-4"
          onSubmit={submit}
        >
          <FieldGroup className="gap-4">
            <div className="grid gap-4 md:grid-cols-3">
              <Field>
                <FieldLabel htmlFor="news-remote-id">远端 ID</FieldLabel>
                <Input
                  id="news-remote-id"
                  value={form.remoteId}
                  disabled={mode === "edit"}
                  onChange={(event) =>
                    setForm((value) => ({
                      ...value,
                      remoteId: event.target.value,
                    }))
                  }
                />
              </Field>
              <GameSelect
                id="news-game-code"
                label="游戏"
                value={form.gameCode}
                disabled={mode === "edit"}
                onChange={(gameCode) =>
                  setForm((value) => ({ ...value, gameCode }))
                }
              />
              <Field>
                <FieldLabel htmlFor="news-source">来源</FieldLabel>
                <Input
                  id="news-source"
                  value={form.source}
                  disabled={mode === "edit"}
                  onChange={(event) =>
                    setForm((value) => ({ ...value, source: event.target.value }))
                  }
                />
              </Field>
            </div>

            <Field>
              <FieldLabel htmlFor="news-title">标题</FieldLabel>
              <Input
                id="news-title"
                value={form.title}
                onChange={(event) =>
                  setForm((value) => ({ ...value, title: event.target.value }))
                }
              />
            </Field>

            <Field>
              <FieldLabel htmlFor="news-intro">正文简介</FieldLabel>
              <Textarea
                id="news-intro"
                className="min-h-32"
                value={form.intro}
                onChange={(event) =>
                  setForm((value) => ({ ...value, intro: event.target.value }))
                }
              />
            </Field>

            <BasicDatePicker
              id="news-publish-time"
              label="发布时间"
              value={form.publishTime}
              onChange={(publishTime) =>
                setForm((value) => ({ ...value, publishTime }))
              }
            />

            <Field>
              <FieldLabel htmlFor="news-source-url">原文链接</FieldLabel>
              <Input
                id="news-source-url"
                value={form.sourceUrl}
                onChange={(event) =>
                  setForm((value) => ({
                    ...value,
                    sourceUrl: event.target.value,
                  }))
                }
              />
            </Field>

            <Field>
              <FieldLabel htmlFor="news-cover">封面</FieldLabel>
              <Input
                id="news-cover"
                value={form.cover}
                onChange={(event) =>
                  setForm((value) => ({ ...value, cover: event.target.value }))
                }
              />
            </Field>

            <div className="grid gap-4 md:grid-cols-[160px_1fr]">
              <Field>
                <FieldLabel>类型</FieldLabel>
                <Select
                  value={form.isVideo}
                  onValueChange={(value) =>
                    setForm((current) => ({
                      ...current,
                      isVideo: value as NewsFormState["isVideo"],
                    }))
                  }
                >
                  <SelectTrigger className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="false">图文</SelectItem>
                    <SelectItem value="true">视频</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
              <Field>
                <FieldLabel htmlFor="news-video-url">视频链接</FieldLabel>
                <Input
                  id="news-video-url"
                  value={form.videoUrl}
                  onChange={(event) =>
                    setForm((value) => ({
                      ...value,
                      videoUrl: event.target.value,
                    }))
                  }
                />
              </Field>
            </div>

            <Field>
              <FieldLabel htmlFor="news-categories">分类</FieldLabel>
              <Input
                id="news-categories"
                value={form.categories}
                placeholder="多个分类用英文逗号分隔"
                onChange={(event) =>
                  setForm((value) => ({
                    ...value,
                    categories: event.target.value,
                  }))
                }
              />
            </Field>

            <Field>
              <FieldLabel htmlFor="news-tags">标签</FieldLabel>
              <Input
                id="news-tags"
                value={form.tags}
                placeholder="多个标签用英文逗号分隔"
                onChange={(event) =>
                  setForm((value) => ({ ...value, tags: event.target.value }))
                }
              />
            </Field>

            {error ? <FieldError>{error}</FieldError> : null}
          </FieldGroup>
        </form>

        <SheetFooter className="border-t">
          <div className="flex items-center justify-between gap-2">
            {mode === "edit" ? (
              <NewsDeleteDialog
                disabled={saving || deleting}
                deleting={deleting}
                onConfirm={deleteNews}
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
                form="news-editor-form"
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

function NewsDeleteDialog({
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
          <AlertDialogTitle>删除这条新闻？</AlertDialogTitle>
          <AlertDialogDescription>
            删除会同时移除新闻和它的分类、标签关联。
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

function createNewsForm(item: NewsItem | null): NewsFormState {
  return {
    remoteId: item?.remote_id ?? "",
    gameCode: item?.game_code ?? "",
    source: item?.source ?? "",
    title: item?.title ?? "",
    intro: item?.intro ?? "",
    publishTime: item?.publish_time ?? "",
    sourceUrl: item?.source_url ?? "",
    cover: item?.cover ?? "",
    isVideo: item?.is_video ? "true" : "false",
    videoUrl: item?.video_url ?? "",
    categories: item?.categories.join(", ") ?? "",
    tags: item?.tags.join(", ") ?? "",
  };
}

function newsPayload(form: NewsFormState, mode: NewsSheetMode) {
  const payload = {
    title: form.title,
    intro: optionalText(form.intro),
    publish_time: form.publishTime,
    source_url: form.sourceUrl,
    cover: form.cover,
    is_video: form.isVideo === "true",
    video_url: optionalText(form.videoUrl),
    categories: parseList(form.categories),
    tags: parseList(form.tags),
  };

  if (mode === "create") {
    return {
      remote_id: form.remoteId,
      game_code: form.gameCode,
      source: form.source,
      ...payload,
    };
  }

  return payload;
}

function newsSaveUrl(mode: NewsSheetMode, item: NewsItem | null) {
  if (mode === "create") {
    return "/admin/data/news";
  }

  if (!item) {
    return "/admin/data/news";
  }

  return newsItemUrl(item);
}

function newsItemUrl(item: NewsItem) {
  return `/admin/data/news/${encodeURIComponent(item.source)}/${encodeURIComponent(
    item.game_code,
  )}/${encodeURIComponent(item.remote_id)}`;
}

function parseList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
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
