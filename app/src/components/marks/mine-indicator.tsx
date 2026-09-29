import { useTranslation } from "react-i18next";
import { Heart } from "lucide-react";

import { STATUS_ICONS } from "@/components/marks/marks-section";
import type { BookSummary, Kind } from "@/lib/api";
import { statusKey } from "@/lib/kinds";

/** My status and favorite next to a title in the catalog, small and quiet (D68). */
export function MineIndicator({
  book,
  activeKind = null,
  large = false,
}: {
  book: BookSummary;
  /** The kind chip selected, if any: shows this kind's status wording instead of the generic one. */
  activeKind?: Kind | null;
  large?: boolean;
}) {
  const { t } = useTranslation();
  const Icon = book.myStatus ? STATUS_ICONS[book.myStatus] : null;
  if (!Icon && !book.myFavorite) return null;
  return (
    <span className="flex shrink-0 items-center gap-1.5 text-muted-foreground">
      {Icon && book.myStatus && (
        <Icon
          role="img"
          aria-label={t("marks.filter.mine", {
            status: t(statusKey(activeKind, book.myStatus)),
          })}
          className={large ? "size-[15px]" : "size-3.5"}
        />
      )}
      {book.myFavorite && (
        <Heart
          role="img"
          aria-label={t("marks.favorite")}
          className={large ? "size-[13px]" : "size-3"}
          fill="currentColor"
        />
      )}
    </span>
  );
}
