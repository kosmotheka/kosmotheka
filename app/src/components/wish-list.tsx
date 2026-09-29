import { useTranslation } from "react-i18next";
import { Bookmark, ShoppingBag, Trash2 } from "lucide-react";

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
      <div className="flex flex-1 flex-col items-center justify-center gap-2.5 pt-10 pb-20 text-center">
        <div className="flex size-13 items-center justify-center rounded-full bg-muted text-muted-foreground">
          <Bookmark className="size-6" />
        </div>
        <p className="mt-1 text-[17px] font-semibold">{t("wishes.empty.title")}</p>
        <p className="max-w-[420px] text-sm text-pretty text-muted-foreground">
          {t("wishes.empty.body")}
        </p>
      </div>
    );
  }

  return (
    <ul className="-mx-4 md:mx-0">
      {wishes.map((wish) => (
        <li key={wish.id} className="flex min-h-16 items-center gap-3.5 px-4 py-2">
          <Cover url={wish.coverUrl} alt="" className="h-14 rounded-md opacity-75" />
          <div className="flex min-w-0 flex-1 flex-col gap-[3px]">
            <span className="truncate text-[15px] font-medium">{wish.title}</span>
            <span className="truncate text-[13px] text-muted-foreground">
              {[wish.author, wish.note].filter(Boolean).join(" · ") || wish.isbn13}
            </span>
          </div>
          <Button
            variant="ghost"
            size="icon-lg"
            className="rounded-full"
            aria-label={t("wishes.bought")}
            title={t("wishes.bought")}
            onClick={() => onBought(wish)}
          >
            <ShoppingBag />
          </Button>
          <ConfirmButton
            trigger={
              <Button
                variant="ghost"
                size="icon-lg"
                className="rounded-full text-muted-foreground"
                aria-label={t("wishes.remove")}
                title={t("wishes.remove")}
              >
                <Trash2 />
              </Button>
            }
            title={t("wishes.removeConfirm", { title: wish.title })}
            description=""
            confirmLabel={t("wishes.remove")}
            onConfirm={() => onRemove(wish)}
          />
        </li>
      ))}
    </ul>
  );
}
