import type { TFunction } from "i18next";

import { type TrashRef, restoreFromTrash } from "@/lib/api";

/** What pressing a floating notice does. */
interface NoticeAction {
  label: string;
  run: () => void;
}

/** Shows the app's floating notice, with an action if it has one; the notice closes when the action runs. */
export type Notify = (text: string, action?: NoticeAction) => void;

/** «Moved to the trash · Undo» after deleting something (D105); undo restores it at once. */
export function notifyTrashed(notify: Notify, t: TFunction, item: TrashRef, onChanged: () => void) {
  notify(t("trash.moved"), {
    label: t("trash.undo"),
    run: () => {
      restoreFromTrash([item])
        .then(onChanged)
        .catch((e: unknown) => notify(String(e)));
    },
  });
}
