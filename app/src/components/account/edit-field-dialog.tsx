import { useState } from "react";
import { useTranslation } from "react-i18next";

import { AuthErrorBlock } from "@/components/account/auth-error-block";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type AccountActionError, accountErrorOf } from "@/lib/account-errors";

/**
 * A small dialog with one text field, e.g. the account screen's «Nombre» row
 * (Account.dc.html). Reused wherever a single value needs its own confirm step.
 */
export function EditFieldDialog({
  open,
  desktop,
  onOpenChange,
  title,
  label,
  value,
  onSave,
}: {
  open: boolean;
  desktop: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  label: string;
  value: string;
  onSave: (value: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(value);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AccountActionError | null>(null);
  const [seeded, setSeeded] = useState(false);

  if (open && !seeded) {
    setSeeded(true);
    setDraft(value);
    setError(null);
  } else if (!open && seeded) {
    setSeeded(false);
  }

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await onSave(draft.trim());
      onOpenChange(false);
    } catch (e) {
      setError(accountErrorOf(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={() => onOpenChange(false)}
      title={title}
      description={title}
    >
      <div className="flex flex-col gap-4">
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium">{label}</span>
          <Input
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            disabled={busy}
          />
        </label>
        {error && <AuthErrorBlock reason={error.reason} retryAfter={error.retryAfter} />}
        <div className="grid grid-cols-2 gap-2.5">
          <Button variant="outline" className="h-11" onClick={() => onOpenChange(false)}>
            {t("form.cancel")}
          </Button>
          <Button className="h-11" disabled={busy || !draft.trim()} onClick={() => void save()}>
            {t("form.save")}
          </Button>
        </div>
      </div>
    </SheetOrDialog>
  );
}
