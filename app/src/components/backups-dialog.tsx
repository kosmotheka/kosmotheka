import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Download,
  FileSpreadsheet,
  FileUp,
  HardDrive,
  Lock,
  TriangleAlert,
  Upload,
} from "lucide-react";
import { open, save } from "@tauri-apps/plugin-dialog";

import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  type CsvImportSummary,
  type RestoreSummary,
  exportBackup,
  exportCsv,
  exportDeviceBackup,
  importCsv,
  isCommandError,
  restoreBackup,
} from "@/lib/api";

const today = () => new Date().toISOString().slice(0, 10);
/** Same minimum as an account password. */
const MIN_PASSPHRASE = 8;
const slug = (name: string) => name.toLocaleLowerCase().replace(/\s+/g, "-");

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
  const [restored, setRestored] = useState<RestoreSummary | null>(null);
  // Encrypted backups are the default (D86); turning it off is a deliberate choice.
  const [protect, setProtect] = useState(true);
  const [passphrase, setPassphrase] = useState("");
  const [confirmation, setConfirmation] = useState("");
  // The encrypted file picked for restoring, while we wait for its password.
  const [locked, setLocked] = useState<string | null>(null);
  const [unlockPassphrase, setUnlockPassphrase] = useState("");
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const library = nameOf(libraries.active);
  const several = libraries.all.length > 1;

  const run = async (action: () => Promise<string | null>) => {
    setBusy(true);
    setMessage(null);
    setRejected([]);
    setRestored(null);
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

  const passphraseOk = passphrase.length >= MIN_PASSPHRASE && passphrase === confirmation;
  const canBackup = !protect || passphraseOk;
  const zip = [{ name: t("data.backupFile"), extensions: ["zip"] }];
  const encrypted = [{ name: t("data.encryptedBackupFile"), extensions: ["kosmotheka"] }];
  const csv = [{ name: "CSV", extensions: ["csv"] }];

  const backup = () =>
    run(async () => {
      const path = await save({
        defaultPath: `${slug(library)}-${today()}${protect ? ".kosmotheka" : ".zip"}`,
        filters: protect ? encrypted : zip,
      });
      if (!path) return null;
      return t("data.backupDone", {
        ...(await exportBackup(path, undefined, protect ? passphrase : undefined)),
      });
    });

  const backupDevice = () =>
    run(async () => {
      const path = await save({
        defaultPath: `kosmotheka-${today()}${protect ? ".kosmotheka" : ".zip"}`,
        filters: protect ? encrypted : zip,
      });
      if (!path) return null;
      return t("data.deviceBackupDone", {
        ...(await exportDeviceBackup(path, protect ? passphrase : undefined)),
      });
    });

  const restore = () =>
    run(async () => {
      const path = await open({
        multiple: false,
        directory: false,
        filters: [...zip, ...encrypted],
      });
      if (!path) return null;
      setLocked(null);
      setUnlockPassphrase("");
      try {
        const summary = await restoreBackup(path);
        onChanged();
        setRestored(summary);
      } catch (e) {
        // An encrypted file: ask for its password instead of failing.
        if (isCommandError(e) && e.kind === "backup" && e.reason === "passphrase_needed") {
          setLocked(path);
          return null;
        }
        throw e;
      }
      return null;
    });

  // A wrong password keeps the file waiting so it can be tried again.
  const unlock = () =>
    run(async () => {
      if (!locked) return null;
      const summary = await restoreBackup(locked, unlockPassphrase);
      onChanged();
      setRestored(summary);
      setLocked(null);
      setUnlockPassphrase("");
      return null;
    });

  const toCsv = () =>
    run(async () => {
      const path = await save({ defaultPath: `${slug(library)}-${today()}.csv`, filters: csv });
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
      <section className="flex flex-col gap-3">
        <label className="flex items-center gap-2.5">
          <Lock className="size-[15px]" />
          <span className="flex-1 text-sm font-medium">{t("data.protect")}</span>
          <Switch
            checked={protect}
            aria-label={t("data.protect")}
            disabled={busy}
            onCheckedChange={setProtect}
          />
        </label>
        {protect ? (
          <>
            <Input
              type="password"
              autoComplete="new-password"
              aria-label={t("data.passphrase")}
              placeholder={t("data.passphrase")}
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              disabled={busy}
            />
            <Input
              type="password"
              autoComplete="new-password"
              aria-label={t("data.passphraseConfirm")}
              placeholder={t("data.passphraseConfirm")}
              value={confirmation}
              onChange={(e) => setConfirmation(e.target.value)}
              disabled={busy}
            />
            {passphrase.length > 0 && passphrase.length < MIN_PASSPHRASE && (
              <p className="text-[13px] text-destructive">
                {t("data.passphraseShort", { min: MIN_PASSPHRASE })}
              </p>
            )}
            {confirmation.length > 0 && passphrase !== confirmation && (
              <p className="text-[13px] text-destructive">{t("data.passphraseMismatch")}</p>
            )}
            <p className="text-[13px] text-pretty text-muted-foreground">{t("data.protectHint")}</p>
          </>
        ) : (
          <p className="flex gap-2 text-[13px] text-pretty text-destructive">
            <TriangleAlert className="mt-0.5 size-[14px] shrink-0" />
            {t("data.unprotectedWarning")}
          </p>
        )}
      </section>
      <div className="grid gap-2">
        <Button className="h-12" disabled={busy || !canBackup} onClick={() => void backup()}>
          <Download /> {t("data.backupOf", { name: library })}
        </Button>
        {several && (
          <Button
            variant="outline"
            className="h-12"
            disabled={busy || !canBackup}
            onClick={() => void backupDevice()}
          >
            <HardDrive /> {t("data.backupDevice", { count: libraries.all.length })}
          </Button>
        )}
        <Button variant="outline" className="h-12" disabled={busy} onClick={() => void restore()}>
          <Upload /> {t("data.restore")}
        </Button>
        <p className="text-[13px] text-pretty text-muted-foreground">{t("data.restoreHint")}</p>
      </div>
      {locked && (
        <form
          className="flex flex-col gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void unlock();
          }}
        >
          <p className="text-sm">{t("data.encryptedFile")}</p>
          <Input
            type="password"
            autoComplete="off"
            autoFocus
            aria-label={t("data.passphrase")}
            placeholder={t("data.passphrase")}
            value={unlockPassphrase}
            onChange={(e) => setUnlockPassphrase(e.target.value)}
            disabled={busy}
          />
          <Button type="submit" disabled={busy || unlockPassphrase.length === 0}>
            <Upload /> {t("data.unlockRestore")}
          </Button>
        </form>
      )}
      {message && <p className="text-sm">{message}</p>}
      {restored && (
        <div className="space-y-1 text-sm">
          {restored.restored.length === 0 && restored.skipped.length === 0 && (
            <p>{t("data.restoreNothing")}</p>
          )}
          {restored.restored.map((item) => (
            <p key={item.id}>
              {t(item.sameId ? "data.restoredLibrary" : "data.restoredNewId", {
                name: nameOf(item),
                books: item.books,
                copies: item.copies,
              })}
            </p>
          ))}
          {restored.skipped.map((name, index) => (
            <p key={`skipped-${index}`} className="text-muted-foreground">
              {t("data.skippedLibrary", { name: nameOf({ name }) })}
            </p>
          ))}
          {restored.covers > 0 && (
            <p className="text-muted-foreground">
              {t("data.restoredCovers", { count: restored.covers })}
            </p>
          )}
        </div>
      )}
      <section className="mt-2 flex flex-col gap-2 border-t pt-5">
        <h3 className="text-sm font-medium">{t("data.csvTitle", { name: library })}</h3>
        <p className="text-sm text-muted-foreground">{t("data.csvDescription")}</p>
        <p className="text-[13px] text-muted-foreground">{t("data.csvReadable")}</p>
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
              {row.errors.map((e) => `${e.field}: ${t(`errors.${e.code}`)}`).join(" · ")}
            </li>
          ))}
        </ul>
      )}
    </ResponsiveDialog>
  );
}
