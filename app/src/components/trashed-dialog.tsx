import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useRequests } from "@/components/requests-context";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { type TrashItem, listTrash, purgeFromTrash, restoreFromTrash } from "@/lib/api";

/**
 * Adding something that is in the trash (D105, point 8): the same code, or a tag or series of the same
 * name. Two alike would be confused, so the choice is to restore it, with what it brings back, or to
 * delete it for good and start over. Deleting before the 30 days is for administrators: an editor only
 * restores or cancels.
 */
export function TrashedDialog({
  item,
  onClose,
  onRestored,
  onStartOver,
}: {
  /** What is in the trash; the dialog is open while it is set. */
  item: TrashItem | null;
  onClose: () => void;
  /** Restored: what was being added is now back in the catalog. */
  onRestored: (item: TrashItem) => void;
  /** Deleted for good: carry on adding the new one. */
  onStartOver: () => void;
}) {
  const { t } = useTranslation();
  const { myRole } = useRequests();
  const admin = myRole === "admin";
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (action: () => Promise<void>, then: () => void) => {
    setBusy(true);
    setError(null);
    try {
      await action();
      then();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const ref = item ? [{ kind: item.kind, id: item.id }] : [];
  const sent = item
    ? item.trashedBy
      ? t("trashed.sentBy", { who: item.trashedBy })
      : t("trashed.sent")
    : "";

  return (
    <AlertDialog open={item !== null} onOpenChange={(open) => !open && !busy && onClose()}>
      <AlertDialogContent>
        {item && (
          <>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {t(`trashed.title.${item.kind}`, { title: item.title })}
              </AlertDialogTitle>
              <AlertDialogDescription className="flex flex-col gap-2">
                <span>
                  {sent}{" "}
                  {item.count > 0 && item.kind !== "copy"
                    ? t(`trashed.back.${item.kind}`, { count: item.count })
                    : t("trashed.back.asItWas")}
                </span>
                <span>{admin ? t("trashed.startOverAdmin") : t("trashed.startOverEditor")}</span>
              </AlertDialogDescription>
            </AlertDialogHeader>
            {error && <p className="text-[13px] text-destructive">{error}</p>}
            <AlertDialogFooter>
              <AlertDialogCancel disabled={busy}>{t("form.cancel")}</AlertDialogCancel>
              {admin && (
                <Button
                  variant="outline"
                  className="text-destructive"
                  disabled={busy}
                  onClick={() => void run(() => purgeFromTrash(ref), onStartOver)}
                >
                  {t("trashed.startOver")}
                </Button>
              )}
              <Button
                disabled={busy}
                onClick={() =>
                  void run(
                    () => restoreFromTrash(ref),
                    () => onRestored(item),
                  )
                }
              >
                {t("trash.restore")}
              </Button>
            </AlertDialogFooter>
          </>
        )}
      </AlertDialogContent>
    </AlertDialog>
  );
}

/** The trash's row for an edition, tag or series found by id, if it is still there. */
export async function trashedItem(kind: TrashItem["kind"], id: string): Promise<TrashItem | null> {
  const items = await listTrash();
  return items.find((item) => item.kind === kind && item.id === id) ?? null;
}
