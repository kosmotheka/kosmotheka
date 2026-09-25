import { useTranslation } from "react-i18next";

import { WishList } from "@/components/wish-list";
import { type Wish, deleteWish } from "@/lib/api";
import { openOverlay } from "@/lib/navigation";

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
    <div className="space-y-4">
      {desktop && <h1 className="text-[22px] font-semibold tracking-tight">{t("nav.missing")}</h1>}
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
    </div>
  );
}
