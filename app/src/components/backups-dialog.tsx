import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Download, FileSpreadsheet, FileUp, Upload } from "lucide-react";
import { open, save } from "@tauri-apps/plugin-dialog";

import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import {
  type CsvImportSummary,
  exportBackup,
  exportCsv,
  importCsv,
  isCommandError,
  restoreBackup,
} from "@/lib/api";

const today = () => new Date().toISOString().slice(0, 10);

/** Backups and CSV files. Every action asks for a file first; cancelling does nothing. */
export function BackupsDialog({
  open: isOpen,
  onOpenChange,
  onChanged,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called when a restore or an import changed the catalog. */
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [rejected, setRejected] = useState<CsvImportSummary["rejected"]>([]);

  const run = async (action: () => Promise<string | null>) => {
    setBusy(true);
    setMessage(null);
    setRejected([]);
    try {
      const result = await action();
      if (result) setMessage(result);
    } catch (e) {
      setMessage(
        isCommandError(e) && e.kind === "backup"
          ? t(`data.backupErrors.${e.reason}`)
          : t("errors.internal", {
              message: isCommandError(e) && e.kind === "internal" ? e.message : String(e),
            }),
      );
    } finally {
      setBusy(false);
    }
  };

  const zip = [{ name: t("data.backupFile"), extensions: ["zip"] }];
  const csv = [{ name: "CSV", extensions: ["csv"] }];

  const backup = () =>
    run(async () => {
      const path = await save({ defaultPath: `kosmotheka-${today()}.zip`, filters: zip });
      if (!path) return null;
      return t("data.backupDone", { ...(await exportBackup(path)) });
    });

  const restore = () =>
    run(async () => {
      const path = await open({ multiple: false, directory: false, filters: zip });
      if (!path) return null;
      const summary = await restoreBackup(path);
      onChanged();
      return t("data.restoreDone", { ...summary });
    });

  const toCsv = () =>
    run(async () => {
      const path = await save({ defaultPath: `kosmotheka-${today()}.csv`, filters: csv });
      if (!path) return null;
      return t("data.csvExported", { count: await exportCsv(path) });
    });

  const fromCsv = () =>
    run(async () => {
      const path = await open({ multiple: false, directory: false, filters: csv });
      if (!path) return null;
      const summary = await importCsv(path);
      onChanged();
      setRejected(summary.rejected);
      return t("data.csvImported", {
        books: summary.booksCreated,
        copies: summary.copiesAdded,
        rejected: summary.rejected.length,
      });
    });

  return (
    <ResponsiveDialog
      open={isOpen}
      onOpenChange={onOpenChange}
      title={t("data.title")}
      description={t("data.description")}
    >
      <div className="grid gap-2">
        <Button className="h-12" disabled={busy} onClick={() => void backup()}>
          <Download /> {t("data.backup")}
        </Button>
        <Button variant="outline" className="h-12" disabled={busy} onClick={() => void restore()}>
          <Upload /> {t("data.restore")}
        </Button>
      </div>
      {message && <p className="text-sm">{message}</p>}
      <section className="mt-2 flex flex-col gap-2 border-t pt-5">
        <h3 className="text-sm font-medium">{t("data.csvTitle")}</h3>
        <p className="text-sm text-muted-foreground">{t("data.csvDescription")}</p>
        <div className="grid grid-cols-2 gap-2">
          <Button variant="outline" disabled={busy} onClick={() => void fromCsv()}>
            <FileUp /> {t("data.importCsv")}
          </Button>
          <Button variant="outline" disabled={busy} onClick={() => void toCsv()}>
            <FileSpreadsheet /> {t("data.exportCsv")}
          </Button>
        </div>
      </section>
      {rejected.length > 0 && (
        <ul className="max-h-40 space-y-1 overflow-y-auto text-sm text-muted-foreground">
          {rejected.map((row) => (
            <li key={row.line}>
              {t("data.rejectedLine", { line: row.line })}{" "}
              {row.errors
                .map((e) => `${e.field}: ${t(`errors.${e.code}`)}`)
                .join(" · ")}
            </li>
          ))}
        </ul>
      )}
    </ResponsiveDialog>
  );
}
