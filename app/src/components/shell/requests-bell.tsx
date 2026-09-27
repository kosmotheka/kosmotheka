import { useTranslation } from "react-i18next";
import { Bell } from "lucide-react";

import { useRequests } from "@/components/requests-context";
import { Button } from "@/components/ui/button";
import { navigate } from "@/lib/navigation";
import { cn } from "@/lib/utils";

/** Administrators only: the requests waiting for them, as a bell with a count. */
export function RequestsBell({ mobile = false }: { mobile?: boolean }) {
  const { t } = useTranslation();
  const { myRole, pending } = useRequests();
  if (myRole !== "admin") return null;
  return (
    <Button
      variant="ghost"
      size={mobile ? "icon-lg" : "icon"}
      className={cn("relative", mobile ? "h-12 w-11 rounded-full" : "size-9")}
      aria-label={t("requests.bell", { count: pending })}
      onClick={() => navigate("requests")}
    >
      <Bell className={mobile ? "size-[21px]" : "size-[18px]"} />
      {pending > 0 && (
        <span
          className={cn(
            "absolute flex items-center justify-center rounded-full bg-destructive font-semibold text-white ring-2 ring-background",
            mobile
              ? "top-2 right-1 h-[18px] min-w-[18px] px-[5px] text-[11px]"
              : "top-0.5 right-0 h-4 min-w-4 px-1 text-[10px]",
          )}
        >
          {pending}
        </span>
      )}
    </Button>
  );
}
