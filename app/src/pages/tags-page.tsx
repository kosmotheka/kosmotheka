import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Pencil, Tag, Trash2 } from "lucide-react";

import { ConfirmButton } from "@/components/confirm-button";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type TagSummary, deleteTag, isCommandError, listTags, renameTag } from "@/lib/api";
import { leaveSection } from "@/lib/navigation";

/** Library settings › Tags (6.7): rename (merging into an existing name) and delete. */
export function TagsPage({
  desktop,
  version,
  onChanged,
}: {
  desktop: boolean;
  version: number;
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const [tags, setTags] = useState<TagSummary[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Only the latest load counts: switching libraries can leave an older one in flight.
  const latest = useRef(0);
  const reload = () => {
    const request = ++latest.current;
    listTags()
      .then((found) => request === latest.current && setTags(found))
      .catch(() => {});
  };
  useEffect(() => {
    reload();
  }, [version]);

  const startRename = (tag: TagSummary) => {
    setEditingId(tag.id);
    setDraft(tag.name);
    setError(null);
  };
  const cancelRename = () => {
    setEditingId(null);
    setError(null);
  };

  const trimmed = draft.trim();
  const merge = tags.find(
    (other) => other.id !== editingId && other.name.toLowerCase() === trimmed.toLowerCase(),
  );

  const saveRename = async (id: string) => {
    try {
      await renameTag(id, draft);
      setEditingId(null);
      reload();
      onChanged();
    } catch (e) {
      const code = isCommandError(e) && e.kind === "validation" ? e.errors[0]?.code : null;
      setError(code ? t(`errors.${code}`) : String(e));
    }
  };

  const remove = async (id: string) => {
    await deleteTag(id);
    reload();
    onChanged();
  };

  const title = t("tags.title");

  return (
    <div className="flex min-h-full flex-col gap-6">
      {desktop ? (
        <div className="flex flex-col gap-0.5">
          <span className="text-[13px] text-muted-foreground">{t("tags.breadcrumb")}</span>
          <h1 className="text-[22px] font-semibold tracking-tight">{title}</h1>
        </div>
      ) : (
        <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("book.back")}
            onClick={leaveSection}
          >
            <ArrowLeft className="size-[22px]" />
          </Button>
          <h1 className="truncate text-lg font-semibold">{title}</h1>
        </header>
      )}

      <p className="max-w-2xl text-sm text-muted-foreground text-pretty">{t("tags.hint")}</p>

      <div className="flex w-full max-w-2xl flex-col border-t">
        {tags.map((tag) => (
          <div key={tag.id} className="flex min-h-13 items-center gap-3 border-b px-2">
            <Tag className="size-[15px] shrink-0 text-muted-foreground" />
            {editingId === tag.id ? (
              <>
                <Input
                  autoFocus
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  className="h-8.5 w-[220px]"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void saveRename(tag.id);
                    if (e.key === "Escape") cancelRename();
                  }}
                />
                {merge && (
                  <span className="text-xs text-muted-foreground">
                    {t("tags.mergeHint", { name: merge.name })}
                  </span>
                )}
                <span className="flex-1" />
                <Button variant="ghost" size="sm" onClick={cancelRename}>
                  {t("form.cancel")}
                </Button>
                <Button size="sm" disabled={!trimmed} onClick={() => void saveRename(tag.id)}>
                  {merge ? t("tags.merge") : t("form.save")}
                </Button>
              </>
            ) : (
              <>
                <span className="flex-1 truncate text-sm font-medium">{tag.name}</span>
                <span className="text-[13px] text-muted-foreground">
                  {t("tags.worksCount", { count: tag.works })}
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t("tags.renameAria", { name: tag.name })}
                  onClick={() => startRename(tag)}
                >
                  <Pencil className="size-[15px]" />
                </Button>
                <ConfirmButton
                  trigger={
                    <Button
                      variant="ghost"
                      size="icon"
                      aria-label={t("tags.deleteAria", { name: tag.name })}
                    >
                      <Trash2 className="size-[15px]" />
                    </Button>
                  }
                  title={t("tags.deleteTitle", { name: tag.name })}
                  description={t("tags.deleteBody", { count: tag.works })}
                  confirmLabel={t("tags.delete")}
                  onConfirm={() => void remove(tag.id)}
                />
              </>
            )}
          </div>
        ))}
        {tags.length === 0 && (
          <p className="py-6 text-center text-sm text-muted-foreground">{t("tags.empty")}</p>
        )}
      </div>
      {error && <p className="text-[13px] text-destructive">{error}</p>}
    </div>
  );
}
