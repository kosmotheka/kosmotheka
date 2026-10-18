import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  ArrowLeft,
  ChevronDown,
  CircleAlert,
  EllipsisVertical,
  Info,
  MonitorSmartphone,
  QrCode,
  RefreshCw,
  ShieldCheck,
  Trash2,
} from "lucide-react";

import { Field } from "@/components/book-form";
import { CloudSharingSection } from "@/components/cloud/cloud-section";
import { ConfirmButton } from "@/components/confirm-button";
import { ShareTargetsDialog } from "@/components/libraries/share-targets-dialog";
import { CannotShare, Tick } from "@/components/libraries/library-checklist";
import { useLibraries, useLibraryName } from "@/components/libraries-context";
import { Avatar } from "@/components/people/role-parts";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  type DeviceSharing,
  type People,
  type Person,
  assignDevice,
  forgetDevice,
  listPeople,
  lanStatus,
  setDeviceName,
  setSharing,
  sharingOverview,
  syncErrorKey,
} from "@/lib/api";
import { leaveSection, openOverlay } from "@/lib/navigation";
import { foreignCloudNotices } from "@/lib/server";
import { syncDevice, useSyncErrors } from "@/lib/sync-errors";
import { cn } from "@/lib/utils";

interface Device extends DeviceSharing {
  /** This device connects to it, so it can sync from here. */
  canSync: boolean;
}

/**
 * The paired devices and which libraries go to each (Design, phase 4 §6). Ticking
 * shares at once; unticking withdraws access after a confirmation (D71).
 */
export function DevicesPage({
  desktop,
  version,
  onChanged,
  onNotice,
}: {
  desktop: boolean;
  /** Changes after pairing or syncing, so the list reloads. */
  version: number;
  onChanged: () => void;
  onNotice: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const nameOf = useLibraryName();
  const errors = useSyncErrors();
  const [devices, setDevices] = useState<Device[] | null>(null);
  const [name, setName] = useState("");
  const [savedName, setSavedName] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [editing, setEditing] = useState<DeviceSharing | null>(null);
  const [unsharing, setUnsharing] = useState<{ device: Device; library: string } | null>(null);
  const [forgetting, setForgetting] = useState<Device | null>(null);
  const [reloads, setReloads] = useState(0);
  const [loadedAt, setLoadedAt] = useState(0);
  const [people, setPeople] = useState<People | null>(null);
  const reload = useCallback(() => setReloads((n) => n + 1), []);

  useEffect(() => {
    let cancelled = false;
    Promise.all([lanStatus(), sharingOverview(), listPeople().catch(() => null)])
      .then(([status, sharing, who]) => {
        if (cancelled) return;
        setPeople(who);
        const servers = new Set(status.servers.map((s) => s.deviceId));
        setDevices(sharing.map((d) => ({ ...d, canSync: servers.has(d.deviceId) })));
        setLoadedAt(Date.now());
        setName(status.deviceName);
        setSavedName(status.deviceName);
      })
      .catch((e: unknown) => !cancelled && onNotice(String(e)));
    return () => {
      cancelled = true;
    };
  }, [version, reloads, onNotice]);

  const act = async (key: string, action: () => Promise<string | null>) => {
    setBusy(key);
    try {
      const message = await action();
      if (message) onNotice(message);
    } catch (e) {
      onNotice(t(syncErrorKey(e)));
    } finally {
      setBusy(null);
      reload();
    }
  };

  const share = (device: Device, library: string) =>
    act(`${device.deviceId}-${library}`, async () => {
      await setSharing(library, device.deviceId, true);
      return null;
    });

  const unshare = (device: Device, library: string) =>
    act(`${device.deviceId}-${library}`, async () => {
      await setSharing(library, device.deviceId, false);
      return null;
    });

  const syncNow = (device: Device) =>
    act(device.deviceId, async () => {
      const report = await syncDevice(device.deviceId);
      onChanged();
      const summary = t("devices.synced", {
        name: device.name,
        received: report.received.received,
        sent: report.sent,
      });
      return [
        summary,
        ...(report.removed.length > 0
          ? [t("sharing.removed", { count: report.removed.length })]
          : []),
        ...foreignCloudNotices(report, t, t("nav.myLibrary")),
      ].join(" ");
    });

  const forget = (device: Device) =>
    act(`forget-${device.deviceId}`, async () => {
      await forgetDevice(device.deviceId);
      return t("devices.forgotten", { name: device.name });
    });

  const libraryName = (id: string) => nameOf(libraries.all.find((l) => l.id === id));

  // Whose each device is, in the active library (D73).
  const owners = people;
  const owner = (device: Device) =>
    owners && (
      <OwnerCell
        people={owners}
        deviceId={device.deviceId}
        compact={!desktop}
        onAssign={(person) =>
          void act(`owner-${device.deviceId}`, async () => {
            await assignDevice(device.deviceId, person.id);
            return t("devices.ownerChanged", { device: device.name, person: person.name });
          })
        }
      />
    );

  const header = desktop ? (
    <div className="flex items-start justify-between gap-4">
      <div className="flex flex-col gap-1">
        <h1 className="text-[22px] font-semibold tracking-tight">{t("devices.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("devices.pageDescription")}</p>
      </div>
      <Button onClick={() => openOverlay({ name: "pair" })}>
        <QrCode /> {t("pair.title")}
      </Button>
    </div>
  ) : (
    <>
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
        <h1 className="truncate text-lg font-semibold">{t("devices.title")}</h1>
      </header>
      <p className="-mt-3 text-[13px] text-muted-foreground">{t("devices.mobileDescription")}</p>
    </>
  );

  const menu = (device: Device) => (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={t("devices.more", { name: device.name })}>
          <EllipsisVertical />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {device.canSync && (
          <DropdownMenuItem disabled={busy !== null} onSelect={() => void syncNow(device)}>
            <RefreshCw /> {t("devices.syncNow")}
          </DropdownMenuItem>
        )}
        <DropdownMenuItem variant="destructive" onSelect={() => setForgetting(device)}>
          <Trash2 /> {t("devices.forget")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );

  const locked = libraries.all.filter((l) => !l.canShare).map((l) => nameOf(l));

  const body =
    devices === null ? null : devices.length === 0 ? (
      <div className="flex flex-col items-start gap-3 rounded-2xl border p-5">
        <p className="text-sm text-pretty text-muted-foreground">{t("devices.none")}</p>
        <Button onClick={() => openOverlay({ name: "pair" })}>
          <QrCode /> {t("pair.title")}
        </Button>
      </div>
    ) : desktop ? (
      <div className="overflow-hidden rounded-2xl border">
        <table className="w-full text-sm">
          <thead className="bg-muted/60 text-xs text-muted-foreground">
            <tr>
              <th className="px-4 py-3 text-left font-normal">{t("devices.device")}</th>
              {owners && (
                <th className="w-52 px-2 py-3 text-left font-normal">{t("devices.owner")}</th>
              )}
              {libraries.all.map((library) => (
                <th key={library.id} className="w-28 px-2 py-3 text-center font-normal">
                  {nameOf(library)}
                </th>
              ))}
              <th className="w-12" />
            </tr>
          </thead>
          <tbody className="divide-y">
            {devices.map((device) => {
              const error = errors[device.deviceId];
              return [
                <tr key={device.deviceId} className={cn(error && "bg-destructive/5")}>
                  <td className="px-4 py-3">
                    <DeviceTitle device={device} error={error} now={loadedAt} />
                  </td>
                  {owners && <td className="px-2">{owner(device)}</td>}
                  {libraries.all.map((library) => {
                    const on = device.libraries.includes(library.id);
                    // Not offered where this device may not share (D73).
                    if (!library.canShare) {
                      return (
                        <td key={library.id} className="px-2 text-center">
                          {on && (
                            <span
                              className="inline-flex size-9 items-center justify-center opacity-50"
                              aria-label={t("devices.cell", {
                                library: nameOf(library),
                                device: device.name,
                              })}
                            >
                              <Tick on />
                            </span>
                          )}
                        </td>
                      );
                    }
                    return (
                      <td key={library.id} className="px-2 text-center">
                        <button
                          type="button"
                          role="checkbox"
                          aria-checked={on}
                          aria-label={t("devices.cell", {
                            library: nameOf(library),
                            device: device.name,
                          })}
                          disabled={busy !== null}
                          className="inline-flex size-9 items-center justify-center rounded-lg hover:bg-muted"
                          onClick={() =>
                            on
                              ? setUnsharing({ device, library: library.id })
                              : void share(device, library.id)
                          }
                        >
                          <Tick on={on} />
                        </button>
                      </td>
                    );
                  })}
                  <td className="pr-2 text-right">{menu(device)}</td>
                </tr>,
                error && (
                  <tr key={`${device.deviceId}-error`} className="bg-destructive/5">
                    <td
                      colSpan={libraries.all.length + (owners ? 3 : 2)}
                      className="px-4 pb-4 pl-16"
                    >
                      <ErrorCard reason={error} />
                    </td>
                  </tr>
                ),
              ];
            })}
          </tbody>
        </table>
      </div>
    ) : (
      <div className="flex flex-col gap-3">
        {devices.map((device) => {
          const error = errors[device.deviceId];
          return (
            <div
              key={device.deviceId}
              className={cn(
                "flex flex-col gap-3 rounded-2xl border p-3.5",
                error && "border-destructive/40 bg-destructive/5",
              )}
            >
              <div className="flex items-center gap-2">
                <div className="min-w-0 flex-1">
                  <DeviceTitle device={device} error={error} now={loadedAt} />
                </div>
                {menu(device)}
              </div>
              {owners && (
                <div className="flex items-center gap-3 border-t pt-3">
                  <span className="w-12 shrink-0 text-xs text-muted-foreground">
                    {t("devices.ownerShort")}
                  </span>
                  <div className="min-w-0 flex-1">{owner(device)}</div>
                </div>
              )}
              {error ? (
                <ErrorCard reason={error} />
              ) : (
                <div className="flex items-end gap-3 border-t pt-3">
                  <div className="flex min-w-0 flex-1 flex-col gap-1.5">
                    <span className="text-xs text-muted-foreground">
                      {t("devices.receivesLabel")}
                    </span>
                    <div className="flex flex-wrap gap-1.5">
                      {device.libraries.length === 0 ? (
                        <span className="text-[13px] text-muted-foreground">
                          {t("sharing.receivesNothing")}
                        </span>
                      ) : (
                        device.libraries.map((id) => (
                          <span
                            key={id}
                            className="rounded-md bg-secondary px-2.5 py-1 text-[13px] font-medium"
                          >
                            {libraryName(id)}
                          </span>
                        ))
                      )}
                    </div>
                  </div>
                  <Button variant="outline" onClick={() => setEditing(device)}>
                    {t("sharing.change")}
                  </Button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    );

  return (
    <div className="flex min-h-full flex-col gap-5">
      {header}
      <div className="flex w-full max-w-4xl flex-col gap-4">
        {locked.length > 0 && <CannotShare libraries={locked} />}
        <CloudSharingSection onChanged={onChanged} onNotice={onNotice} />
        {body}
        {devices && devices.length > 0 && (
          <p className="flex items-start gap-2 text-[13px] text-pretty text-muted-foreground">
            <ShieldCheck className="mt-0.5 size-4 shrink-0" />
            {t("devices.footnote")}
          </p>
        )}
        {owners && devices && devices.length > 0 && (
          <p className="flex items-start gap-2 text-[13px] text-pretty text-muted-foreground">
            <Info className="mt-0.5 size-4 shrink-0" />
            {owners.myRole === "admin"
              ? t("devices.ownerFootnote")
              : t("devices.ownerReadOnly", { library: nameOf(libraries.active) })}
          </p>
        )}
        {!desktop && devices && devices.length > 0 && (
          <Button className="h-12" onClick={() => openOverlay({ name: "pair" })}>
            <QrCode /> {t("devices.pairAnother")}
          </Button>
        )}
        <section className="mt-2 max-w-md border-t pt-5">
          <Field id="device-name" label={t("devices.thisDevice")}>
            <div className="flex gap-2">
              <Input id="device-name" value={name} onChange={(e) => setName(e.target.value)} />
              <Button
                variant="outline"
                disabled={!name.trim() || name.trim() === savedName}
                onClick={() =>
                  void act("name", async () => {
                    await setDeviceName(name);
                    return null;
                  })
                }
              >
                {t("form.save")}
              </Button>
            </div>
          </Field>
        </section>
      </div>

      <ShareTargetsDialog
        device={editing}
        desktop={desktop}
        onClose={() => setEditing(null)}
        onSaved={(message) => {
          onNotice(message);
          reload();
        }}
      />
      <ConfirmButton
        open={unsharing !== null}
        onOpenChange={(open) => !open && setUnsharing(null)}
        title={t("sharing.stopTitle", {
          library: unsharing ? libraryName(unsharing.library) : "",
          device: unsharing?.device.name ?? "",
        })}
        description={t("sharing.stopBody", {
          library: unsharing ? libraryName(unsharing.library) : "",
          device: unsharing?.device.name ?? "",
        })}
        confirmLabel={t("sharing.stop")}
        onConfirm={() => unsharing && void unshare(unsharing.device, unsharing.library)}
      />
      <ConfirmButton
        open={forgetting !== null}
        onOpenChange={(open) => !open && setForgetting(null)}
        title={t("devices.forgetConfirm.title", { name: forgetting?.name ?? "" })}
        description={t("devices.forgetConfirm.body")}
        confirmLabel={t("devices.forget")}
        onConfirm={() => forgetting && void forget(forgetting)}
      />
    </div>
  );
}

function DeviceTitle({
  device,
  error,
  now,
}: {
  device: Device;
  error: string | undefined;
  /** When the list was loaded; "2 minutes ago" counts from then. */
  now: number;
}) {
  const { t, i18n } = useTranslation();
  const since = (at: string) => {
    const minutes = Math.max(0, Math.round((now - new Date(at).getTime()) / 60000));
    const rtf = new Intl.RelativeTimeFormat(i18n.resolvedLanguage, { numeric: "auto" });
    return minutes < 60
      ? rtf.format(-minutes, "minute")
      : minutes < 60 * 48
        ? rtf.format(-Math.round(minutes / 60), "hour")
        : rtf.format(-Math.round(minutes / 1440), "day");
  };
  return (
    <div className="flex items-center gap-3">
      <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-secondary">
        <MonitorSmartphone className="size-[18px]" />
      </span>
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="truncate text-[15px] font-medium">{device.name}</span>
        {error ? (
          <span className="flex items-center gap-1 text-[13px] font-medium text-destructive">
            <CircleAlert className="size-3.5" /> {t("devices.cannotSync")}
          </span>
        ) : (
          <span className="flex items-center gap-1.5 text-[13px] text-muted-foreground">
            <span
              className={cn(
                "size-1.5 rounded-full",
                device.lastSyncAt ? "bg-success" : "bg-muted-foreground/60",
              )}
              aria-hidden
            />
            {device.lastSyncAt
              ? t("devices.upToDate", { when: since(device.lastSyncAt) })
              : t("devices.neverSynced")}
          </span>
        )}
      </span>
    </div>
  );
}

function ErrorCard({ reason }: { reason: string }) {
  const { t } = useTranslation();
  const [details, setDetails] = useState(false);
  return (
    <div className="flex flex-col gap-1.5 rounded-xl border border-destructive/40 bg-background p-3.5 text-sm">
      <p className="text-pretty">{t(`devices.errors.${reason}`)}</p>
      <p className="text-[13px] text-pretty text-muted-foreground">{t("devices.errorHint")}</p>
      <button
        type="button"
        className="flex items-center gap-1 self-start text-[13px] text-muted-foreground"
        aria-expanded={details}
        onClick={() => setDetails(!details)}
      >
        {t("devices.technical")}
        <ChevronDown className={cn("size-3.5", details && "rotate-180")} />
      </button>
      {details && <code className="text-xs text-muted-foreground">sync: {reason}</code>}
    </div>
  );
}

/** Whose a device is; administrators change it from here. */
function OwnerCell({
  people,
  deviceId,
  compact,
  onAssign,
}: {
  people: People;
  deviceId: string;
  compact: boolean;
  onAssign: (person: Person) => void;
}) {
  const { t } = useTranslation();
  const person = people.people.find((p) => p.devices.some((d) => d.id === deviceId));
  const me = person?.devices.find((d) => d.id === deviceId)?.thisDevice;
  const others = people.people.filter((p) => p.id !== person?.id);
  const admin = people.myRole === "admin";
  return (
    <div className="flex min-w-0 items-center gap-2">
      {person ? (
        <>
          <Avatar name={person.name} size={24} />
          <span className="truncate text-sm font-medium">
            {person.name}
            {me && <span className="font-normal text-muted-foreground"> · {t("people.you")}</span>}
          </span>
        </>
      ) : (
        <span className="flex min-w-0 flex-col">
          <span className="text-sm text-muted-foreground">{t("devices.nobody")}</span>
          <span className="text-xs text-muted-foreground">{t("people.countsAsReader")}</span>
        </span>
      )}
      {admin && others.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant={person ? "ghost" : "outline"}
              size="sm"
              className={cn(compact && "ml-auto")}
              aria-label={t("devices.changeOwner")}
            >
              {person ? t("sharing.change") : t("people.assign")}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {others.map((other) => (
              <DropdownMenuItem key={other.id} onSelect={() => onAssign(other)}>
                <Avatar name={other.name} size={20} /> {other.name}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
}
