import { useTranslation } from "react-i18next";
import { BookPlus, Trash2 } from "lucide-react";

import { ConfirmButton } from "@/components/confirm-button";
import { Cover } from "@/components/cover";
import { Button } from "@/components/ui/button";
import type { Wish } from "@/lib/api";

export function WishList({
  wishes,
  onRemove,
  onBought,
}: {
  wishes: Wish[];
  onRemove: (wish: Wish) => void;
  /** Opens "Add book" prefilled; the wish is removed once the book is saved. */
  onBought: (wish: Wish) => void;
}) {
  const { t } = useTranslation();

  if (wishes.length === 0) {
    return (
      <div className="rounded-lg border border-dashed p-12 text-center">
        <p className="font-medium">{t("wishes.empty.title")}</p>
        <p className="text-sm text-muted-foreground">{t("wishes.empty.body")}</p>
      </div>
    );
  }

  return (
    <ul className="grid gap-3 sm:grid-cols-2">
      {wishes.map((wish) => (
        <li key={wish.id} className="flex gap-3 rounded-lg border p-3">
          <Cover url={wish.coverUrl} alt="" className="h-20 w-14" />
          <div className="min-w-0 flex-1 space-y-1 text-sm">
            <p className="font-medium">{wish.title}</p>
            {wish.author && <p className="text-muted-foreground">{wish.author}</p>}
            {wish.isbn13 && <p className="font-mono text-xs text-muted-foreground">{wish.isbn13}</p>}
            {wish.note && <p>{wish.note}</p>}
            <div className="flex gap-1 pt-1">
              <Button size="sm" variant="outline" onClick={() => onBought(wish)}>
                <BookPlus /> {t("wishes.bought")}
              </Button>
              <ConfirmButton
                trigger={
                  <Button size="icon" variant="ghost" aria-label={t("wishes.remove")}>
                    <Trash2 />
                  </Button>
                }
                title={t("wishes.removeConfirm", { title: wish.title })}
                description=""
                confirmLabel={t("wishes.remove")}
                onConfirm={() => onRemove(wish)}
              />
            </div>
          </div>
        </li>
      ))}
    </ul>
  );
}
