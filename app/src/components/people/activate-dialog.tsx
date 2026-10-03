import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Info, ShieldCheck, Smartphone } from "lucide-react";

import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type DeviceOwner,
  type KnownPerson,
  type People,
  type PersonDevice,
  type Role,
  knownPeople,
  permissionErrorKey,
  roles,
  startRoles,
} from "@/lib/api";

interface Row {
  device: PersonDevice;
  /** `none`, `new`, or the id of an existing or known person. */
  pick: string;
  newName: string;
  role: Role;
}

/**
 * Step 2 of turning roles on (Design 04b, D73): whose each device already sharing
 * the library is, and with which role. Editor comes ticked so nobody loses what
 * they could do; a device without a person counts as a reader.
 */
export function ActivateDialog({
  open,
  people,
  library,
  onClose,
  onDone,
}: {
  open: boolean;
  people: People;
  library: string;
  onClose: () => void;
  onDone: (message: string) => void;
}) {
  const { t } = useTranslation();
  const devices = [...people.people.flatMap((p) => p.devices), ...people.unassigned];
  const me = devices.find((d) => d.thisDevice);
  const mine = people.people.find((p) => p.devices.some((d) => d.thisDevice));
  const [myName, setMyName] = useState(mine?.name ?? "");
  const [known, setKnown] = useState<KnownPerson[]>([]);
  const [rows, setRows] = useState<Row[]>(() =>
    devices
      .filter((d) => !d.thisDevice)
      .map((device) => ({
        device,
        // Nobody should lose what they could do: a person with editor unless the
        // administrator chooses "no person" on purpose.
        pick: people.people.find((p) => p.devices.some((d) => d.id === device.id))?.id ?? "new",
        newName: "",
        role: "editor",
      })),
  );
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open)
      knownPeople()
        .then(setKnown)
        .catch(() => setKnown([]));
  }, [open]);

  const update = (index: number, change: Partial<Row>) =>
    setRows(rows.map((row, i) => (i === index ? { ...row, ...change } : row)));

  const nameOf = (pick: string) =>
    people.people.find((p) => p.id === pick)?.name ?? known.find((k) => k.id === pick)?.name ?? "";

  const ready =
    myName.trim() !== "" && rows.every((row) => row.pick !== "new" || row.newName.trim() !== "");

  const activate = async () => {
    setBusy(true);
    setError(null);
    const owners: DeviceOwner[] = rows.map((row) => ({
      deviceId: row.device.id,
      person:
        row.pick === "none"
          ? null
          : row.pick === "new"
            ? { id: null, name: row.newName.trim() }
            : { id: row.pick, name: nameOf(row.pick) },
      role: row.pick === "none" ? null : row.role,
    }));
    try {
      await startRoles(myName.trim(), owners);
      onDone(t("people.activated", { library }));
    } catch (e) {
      const key = permissionErrorKey(e);
      setError(key ? t(key) : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={(next) => !next && onClose()}
      title={t("people.activate.title")}
      description={t("people.activate.description")}
      className="sm:max-w-[680px]"
    >
      <p className="-mt-2 text-[12.5px] text-muted-foreground">
        {t("people.activate.step", { library })}
      </p>
      <div className="flex flex-col divide-y rounded-2xl border sm:rounded-xl">
        {me && (
          <div className="flex flex-col gap-2.5 p-3 sm:grid sm:grid-cols-[minmax(0,1fr)_210px_150px] sm:items-center sm:gap-3">
            <DeviceName device={me} />
            <Input
              aria-label={t("people.activate.yourName")}
              placeholder={t("people.activate.yourName")}
              value={myName}
              onChange={(e) => setMyName(e.target.value)}
            />
            <span className="px-2.5 text-sm">{t("people.roles.admin")}</span>
          </div>
        )}
        {rows.map((row, index) => (
          <div
            key={row.device.id}
            className="flex flex-col gap-2.5 p-3 sm:grid sm:grid-cols-[minmax(0,1fr)_210px_150px] sm:items-start sm:gap-3"
          >
            <DeviceName device={row.device} />
            <div className="flex flex-col gap-2">
              <Select value={row.pick} onValueChange={(pick) => update(index, { pick })}>
                <SelectTrigger
                  className="w-full"
                  aria-label={t("people.activate.owner", { device: row.device.name })}
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">{t("people.activate.nobody")}</SelectItem>
                  {people.people
                    .filter((p) => !p.devices.some((d) => d.thisDevice))
                    .map((p) => (
                      <SelectItem key={p.id} value={p.id}>
                        {p.name}
                      </SelectItem>
                    ))}
                  {known.map((k) => (
                    <SelectItem key={k.id} value={k.id}>
                      {k.name} · {t("people.alreadyIn", { libraries: k.libraries.join(", ") })}
                    </SelectItem>
                  ))}
                  <SelectItem value="new">{t("people.activate.newPerson")}</SelectItem>
                </SelectContent>
              </Select>
              {row.pick === "new" && (
                <Input
                  autoFocus
                  aria-label={t("people.name")}
                  placeholder={t("people.name")}
                  value={row.newName}
                  onChange={(e) => update(index, { newName: e.target.value })}
                />
              )}
              {row.pick === "none" && (
                <span className="text-[11.5px] text-muted-foreground">
                  {t("people.countsAsReader")}
                </span>
              )}
            </div>
            <Select
              value={row.pick === "none" ? "reader" : row.role}
              disabled={row.pick === "none"}
              onValueChange={(role) => update(index, { role: role as Role })}
            >
              <SelectTrigger
                className="w-full"
                aria-label={t("people.activate.role", { device: row.device.name })}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {roles.map((role) => (
                  <SelectItem key={role} value={role}>
                    {t(`people.roles.${role}`)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ))}
      </div>
      <p className="flex gap-2.5 rounded-xl bg-muted px-3.5 py-3 text-[13.5px] leading-snug text-pretty">
        <Info className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <span>
          {t("people.activate.samePeople", { library })}{" "}
          <strong className="font-semibold">{t("people.activate.nobodyIsReader")}</strong>
        </span>
      </p>
      {error && <p className="text-sm text-destructive">{error}</p>}
      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={onClose}>
          {t("common.back")}
        </Button>
        <Button disabled={!ready || busy} onClick={() => void activate()}>
          <ShieldCheck /> {t("people.activate.confirm")}
        </Button>
      </div>
    </ResponsiveDialog>
  );
}

function DeviceName({ device }: { device: PersonDevice }) {
  const { t } = useTranslation();
  return (
    <div className="flex min-w-0 items-center gap-2.5">
      <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-secondary">
        <Smartphone className="size-4" />
      </span>
      <span className="flex min-w-0 flex-col">
        <span className="truncate text-sm font-medium">{device.name}</span>
        {device.thisDevice && (
          <span className="text-xs text-muted-foreground">{t("people.thisDevice")}</span>
        )}
      </span>
    </div>
  );
}
