import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Send, Trash2 } from "lucide-react";

import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { Button } from "@/components/ui/button";
import { permissionErrorKey } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * «Ask to delete» (Design, phase 5 §4, D73): what would go, why it needs asking,
 * and an optional note for whoever decides.
 */
export function AskDialog({
  open,
  desktop,
  title,
  what,
  why,
  onClose,
  onSend,
}: {
  open: boolean;
  desktop: boolean;
  title: string;
  /** What the request would change, in a few words. */
  what: string;
  why: string;
  onClose: () => void;
  onSend: (note: string | null) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (!open) return null;

  const send = async () => {
    setBusy(true);
    setError(null);
    try {
      await onSend(note.trim() || null);
      setNote("");
    } catch (e) {
      const key = permissionErrorKey(e);
      setError(key ? t(key) : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <SheetOrDialog
      open
      desktop={desktop}
      onClose={onClose}
      title={title}
      description={why}
      icon={
        <span className="flex size-10 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive">
          <Trash2 className="size-5" />
        </span>
      }
    >
      <p className="rounded-xl bg-muted px-3.5 py-3 text-sm font-medium">{what}</p>
      <NoteField value={note} onChange={setNote} placeholder={t("ask.deletePlaceholder")} />
      <p className="text-[13px] leading-snug text-pretty text-muted-foreground">{why}</p>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <div className={cn("flex gap-2", desktop ? "justify-end" : "flex-col-reverse")}>
        <Button variant="outline" className={cn(!desktop && "h-12")} onClick={onClose}>
          {t("common.cancel")}
        </Button>
        <Button className={cn(!desktop && "h-12")} disabled={busy} onClick={() => void send()}>
          <Send /> {t("ask.send")}
        </Button>
      </div>
    </SheetOrDialog>
  );
}

/** The optional note for whoever decides a request. */
export function NoteField({
  value,
  onChange,
  id = "request-note",
  placeholder,
}: {
  value: string;
  onChange: (note: string) => void;
  id?: string;
  placeholder?: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {t("ask.note")}{" "}
        <span className="font-normal text-muted-foreground">{t("ask.optional")}</span>
      </label>
      <textarea
        id={id}
        rows={3}
        maxLength={500}
        placeholder={placeholder ?? t("ask.notePlaceholder")}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full resize-none rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
      />
    </div>
  );
}
