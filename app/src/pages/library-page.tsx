import { type SubmitEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  ChevronRight,
  Cloud,
  Copy,
  History,
  LogOut,
  MonitorSmartphone,
  Plus,
  Tag,
  Trash2,
} from "lucide-react";

import { AutolinkSwitch } from "@/components/cloud/autolink-switch";
import { CloudStateLine, useCloudRows } from "@/components/cloud/cloud-state";
import { ConfirmButton } from "@/components/confirm-button";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { useRequests } from "@/components/requests-context";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type KnownDevice,
  isCommandError,
  libraryDevices,
  renameLibrary,
  setSharing,
} from "@/lib/api";
import { leaveSection, navigate, openOverlay } from "@/lib/navigation";

/** Settings of the active library: name, who it is shared with, leave or delete it. */
export function LibraryPage({ desktop }: { desktop: boolean }) {
  const { t, i18n } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const library = libraries.active;
  const [name, setName] = useState(library?.name ?? "");
  const [shownId, setShownId] = useState(library?.id);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [devices, setDevices] = useState<KnownDevice[]>([]);
  const [devicesVersion, setDevicesVersion] = useState(0);
  // Only where it is the way out: a library with roles this device cannot administer.
  const { hasRoles, myRole } = useRequests();
  const mayDuplicate = hasRoles && myRole !== "admin";
  const cloudRow = useCloudRows().find((row) => row.library.id === library?.id);

  // Another library became active: start from its name.
  if (library && shownId !== library.id) {
    setShownId(library.id);
    setName(library.name);
    setError(null);
  }

  useEffect(() => {
    if (!library) return;
    let cancelled = false;
    libraryDevices(library.id)
      .then((found) => !cancelled && setDevices(found))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [library, devicesVersion]);

  if (!library) return null;

  const save = async (event: SubmitEvent) => {
    event.preventDefault();
    setError(null);
    try {
      await renameLibrary(library.id, name);
      setSaved(true);
      libraries.reload();
    } catch (e) {
      const code = isCommandError(e) && e.kind === "validation" ? e.errors[0]?.code : null;
      setError(
        code === "name_in_use"
          ? t("libraries.nameInUse", { name: name.trim() })
          : code
            ? t(`errors.${code}`)
            : String(e),
      );
    }
  };

  const title = t("libraries.settingsOf", { name: nameOf(library) });
  const created = library.created;

  return (
    <div className="flex min-h-full flex-col gap-6">
      {desktop ? (
        <h1 className="text-[22px] font-semibold tracking-tight">{title}</h1>
      ) : (
        <header className="-mx-4 -mt-1 flex h-16 items-center gap-1 px-1">
          <Button
            variant="ghost"
            size="icon-lg"
            className="size-12 rounded-full"
            aria-label={t("book.back")}
            onClick={leaveSection}
          >
            <ArrowLeft className="size-[22px]" />
          </Button>
          <h1 className="truncate text-lg font-semibold">{title}</h1>
        </header>
      )}

      <div className="flex w-full max-w-2xl flex-col gap-6">
        <form onSubmit={(e) => void save(e)} noValidate className="flex flex-col gap-2">
          <label htmlFor="library-rename" className="text-[13px] font-medium text-muted-foreground">
            {t("libraries.name")}
          </label>
          <div className="flex gap-2">
            <Input
              id="library-rename"
              className="h-11"
              placeholder={t("nav.myLibrary")}
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setSaved(false);
                setError(null);
              }}
            />
            <Button
              type="submit"
              variant="outline"
              className="h-11"
              disabled={name === library.name}
            >
              {t("form.save")}
            </Button>
          </div>
          {error && <p className="text-[13px] text-destructive">{error}</p>}
          {saved && !error && (
            <p className="text-[13px] text-muted-foreground">{t("libraries.renamed")}</p>
          )}
        </form>

        <section className="flex flex-col gap-1">
          <div className="flex flex-col gap-0.5 pb-1.5">
            <span className="text-[13px] font-medium text-muted-foreground">
              {t("libraries.sharedWith")}
            </span>
            <span className="text-xs text-muted-foreground">
              {t("libraries.sharedCount", { count: devices.length })}
            </span>
          </div>
          <div className="flex flex-col overflow-hidden rounded-2xl border">
            {devices.map((device) => (
              <div
                key={device.deviceId}
                className="flex min-h-16 items-center gap-3 border-b px-3.5 py-2"
              >
                <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-secondary">
                  <MonitorSmartphone className="size-[19px]" />
                </span>
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="truncate text-[15px] font-medium">{device.name}</span>
                  <span className="text-[13px] text-muted-foreground">
                    {device.lastSyncAt
                      ? t("devices.lastSync", {
                          date: new Date(device.lastSyncAt).toLocaleString(i18n.language),
                        })
                      : t("devices.neverSynced")}
                  </span>
                </span>
                <ConfirmButton
                  trigger={
                    <Button variant="ghost" size="sm" className="shrink-0">
                      {t("sharing.stop")}
                    </Button>
                  }
                  title={t("sharing.stopTitle", { library: nameOf(library), device: device.name })}
                  description={t("sharing.stopBody", {
                    library: nameOf(library),
                    device: device.name,
                  })}
                  confirmLabel={t("sharing.stop")}
                  onConfirm={() =>
                    void setSharing(library.id, device.deviceId, false).then(() =>
                      setDevicesVersion((v) => v + 1),
                    )
                  }
                />
              </div>
            ))}
            <button
              type="button"
              className="flex min-h-14 items-center gap-3 px-3.5 text-left text-[15px] font-medium hover:bg-muted"
              onClick={() => openOverlay({ name: "pair" })}
            >
              <Plus className="size-[19px]" /> {t("libraries.shareWithAnother")}
            </button>
          </div>
          {created && (
            <span className="flex items-center gap-1.5 px-0.5 pt-2 text-xs text-muted-foreground">
              <History className="size-3.5" />
              {created.device
                ? t("libraries.createdOn", {
                    device: created.device,
                    date: new Date(created.at).toLocaleDateString(i18n.language),
                  })
                : t("libraries.createdAt", {
                    date: new Date(created.at).toLocaleDateString(i18n.language),
                  })}
            </span>
          )}
        </section>

        {cloudRow && (
          <section className="flex flex-col gap-1">
            <span className="pb-1.5 text-[13px] font-medium text-muted-foreground">
              {t("cloud.library.title")}
            </span>
            <div className="flex min-h-16 items-center gap-3 rounded-2xl border px-3.5 py-2">
              <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-secondary">
                <Cloud className="size-[19px]" />
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="text-[15px] font-medium">{t("cloud.sharing.title")}</span>
                <CloudStateLine row={cloudRow} />
              </span>
              {(cloudRow.kind === "notUploaded" || cloudRow.kind === "unsigned") && (
                <Button
                  variant="outline"
                  size="sm"
                  className="shrink-0"
                  onClick={() => openOverlay({ name: "cloudPick" })}
                >
                  {t(cloudRow.kind === "unsigned" ? "cloud.row.fix" : "cloud.row.upload")}
                </Button>
              )}
            </div>
          </section>
        )}

        {cloudRow?.status.linkedHere && cloudRow.status.hasRoles && (
          <AutolinkSwitch status={cloudRow.status} library={nameOf(library)} desktop={desktop} />
        )}

        <section className="flex flex-col border-t pt-2">
          <button
            type="button"
            className="flex min-h-14 items-center gap-3.5 rounded-xl px-1 py-2.5 text-left hover:bg-muted"
            onClick={() => navigate("tags")}
          >
            <Tag className="size-5 shrink-0" />
            <span className="flex flex-1 flex-col gap-0.5">
              <span className="text-[15px] font-medium">{t("tags.title")}</span>
              <span className="text-[13px] text-muted-foreground">{t("tags.hint")}</span>
            </span>
            <ChevronRight className="size-4 shrink-0 text-muted-foreground" />
          </button>
        </section>

        <section className="mt-2 flex flex-col border-t pt-2">
          {mayDuplicate && (
            <button
              type="button"
              className="flex min-h-16 items-center gap-3.5 rounded-xl px-1 py-2.5 text-left hover:bg-muted"
              onClick={() => openOverlay({ name: "duplicateLibrary" })}
            >
              <Copy className="size-5 shrink-0" />
              <span className="flex flex-col gap-0.5">
                <span className="text-[15px] font-medium">{t("libraries.duplicate")}</span>
                <span className="text-[13px] text-muted-foreground">
                  {t("libraries.duplicateHint")}
                </span>
              </span>
            </button>
          )}
          <button
            type="button"
            className="flex min-h-16 items-center gap-3.5 rounded-xl px-1 py-2.5 text-left hover:bg-muted"
            onClick={() => openOverlay({ name: "leaveLibrary" })}
          >
            <LogOut className="size-5 shrink-0" />
            <span className="flex flex-col gap-0.5">
              <span className="text-[15px] font-medium">{t("libraries.leave")}</span>
              <span className="text-[13px] text-muted-foreground">{t("libraries.leaveHint")}</span>
            </span>
          </button>
          <button
            type="button"
            className="flex min-h-16 items-center gap-3.5 rounded-xl px-1 py-2.5 text-left text-destructive hover:bg-muted"
            onClick={() => openOverlay({ name: "deleteLibrary" })}
          >
            <Trash2 className="size-5 shrink-0" />
            <span className="flex flex-col gap-0.5">
              <span className="text-[15px] font-semibold">{t("libraries.delete")}</span>
              <span className="text-[13px] text-muted-foreground">{t("libraries.deleteHint")}</span>
            </span>
          </button>
        </section>
      </div>
    </div>
  );
}
