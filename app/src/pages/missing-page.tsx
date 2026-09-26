import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";

import { WishList } from "@/components/wish-list";
import { Button } from "@/components/ui/button";
import { type Wish, deleteWish } from "@/lib/api";
import { navigate, openOverlay } from "@/lib/navigation";

/**
 * "Missing": for now, the wishlist (D36). Series gaps and the owned / missing /
 * wished states arrive with the general catalog (roadmap phase 6, D61).
 */
export function MissingPage({
  wishes,
  desktop,
  onChanged,
}: {
  wishes: Wish[];
  desktop: boolean;
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex min-h-full flex-col gap-3">
      {desktop ? (
        <div className="flex items-center gap-2.5">
          <h1 className="text-[22px] font-semibold tracking-tight">{t("nav.missing")}</h1>
          <span className="text-[13px] text-muted-foreground">
            {t("nav.works", { count: wishes.length })}
          </span>
          <Button className="ml-auto" onClick={() => navigate("check")}>
            <Plus /> {t("wishes.note")}
          </Button>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">{t("nav.works", { count: wishes.length })}</p>
      )}
      <WishList
        wishes={wishes}
        onRemove={(wish) => void deleteWish(wish.id).then(onChanged)}
        onBought={(wish) =>
          openOverlay({
            name: "add",
            prefill: {
              isbn: wish.isbn13 ?? undefined,
              title: wish.title,
              author: wish.author ?? undefined,
            },
            wishId: wish.id,
          })
        }
      />
      {!desktop && (
        // Until wishes can be written by hand (phase 6), noting one goes through "Do I have it?".
        <Button
          className="fixed right-4 bottom-[calc(112px+env(safe-area-inset-bottom))] h-14 gap-2.5 rounded-2xl pr-5 pl-4 text-[15px] font-semibold shadow-[0_8px_20px_-6px_oklch(0_0_0/35%)]"
          onClick={() => navigate("check")}
        >
          <Plus className="size-[22px]" /> {t("wishes.note")}
        </Button>
      )}
    </div>
  );
}
