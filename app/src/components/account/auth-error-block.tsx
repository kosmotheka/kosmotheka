import { useTranslation } from "react-i18next";
import { CircleAlert, Info, TriangleAlert } from "lucide-react";

import { accountErrorText, retryAfterMinutes } from "@/lib/account-errors";
import { cn } from "@/lib/utils";

/**
 * The block under the fields on every auth screen (Auth.dc.html): red for an
 * error, amber for a warning, muted for an info notice. `onAction` does what the
 * design's link says (resend a code, go to «forgot», retry…); omitted hides it.
 */
export function AuthErrorBlock({
  reason,
  retryAfter,
  totpStep,
  onAction,
}: {
  reason: string;
  retryAfter?: number | null;
  /** Picks between the two `invalid_two_factor_code` texts (D77). */
  totpStep?: boolean;
  onAction?: () => void;
}) {
  const { t } = useTranslation();
  const info = accountErrorText(reason, totpStep);
  const level = info?.level ?? "error";
  const Icon = level === "warning" ? TriangleAlert : level === "info" ? Info : CircleAlert;
  const minutes = retryAfterMinutes(retryAfter);

  return (
    <div
      role={level === "error" ? "alert" : "status"}
      className={cn(
        "flex gap-3 rounded-xl p-3.5",
        level === "warning" && "bg-warning-soft",
        level === "info" && "bg-muted",
        level === "error" && "border border-destructive/30 bg-destructive/[0.09]",
      )}
    >
      <Icon
        className={cn(
          "mt-0.5 size-[18px] shrink-0",
          level === "warning" && "text-warning",
          level === "info" && "text-muted-foreground",
          level === "error" && "text-destructive",
        )}
      />
      <div className="flex flex-col gap-1 text-sm leading-snug text-pretty">
        <strong className="font-semibold">
          {info ? t(info.titleKey, { minutes }) : t("errors.internal", { message: reason })}
        </strong>
        {info?.helpKey && (
          <span className="text-muted-foreground">{t(info.helpKey, { minutes })}</span>
        )}
        {info?.actionKey && onAction && (
          <button
            type="button"
            className="mt-0.5 self-start text-left font-semibold text-primary"
            onClick={onAction}
          >
            {t(info.actionKey)}
          </button>
        )}
      </div>
    </div>
  );
}
