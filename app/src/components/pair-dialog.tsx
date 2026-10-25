import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import QRCode from "qrcode";
import {
  ClipboardPaste,
  Copy,
  KeyRound,
  Loader2,
  MonitorSmartphone,
  QrCode,
  ScanLine,
} from "lucide-react";

import { GetTheApp, GetTheAppButton } from "@/components/get-the-app";
import { LibraryChecklist, SendSummary } from "@/components/libraries/library-checklist";
import { useLibraries } from "@/components/libraries-context";
import { OwnerStep } from "@/components/people/owner-step";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  type PairingOwners,
  closePairing,
  isMobile,
  lanStatus,
  openPairing,
  pairWith,
  pairWithPin,
  findPairingDevices,
  type NearbyDevice,
  pairingOwners,
  permissionErrorKey,
  syncErrorKey,
} from "@/lib/api";
import { closeOverlay } from "@/lib/navigation";
import { CameraDenied, scanQr } from "@/lib/scanner";
import { foreignCloudNotices } from "@/lib/server";
import { syncDevice } from "@/lib/sync-errors";
import { cn } from "@/lib/utils";

type Mode = "show" | "enter";

interface Shown {
  code: string;
  /** The six digits to pair by hand (D88). */
  pin: string;
  qr: string;
  expires: number;
}

/**
 * Pairing with another device: choose which libraries go to it, then show this
 * device's code or enter the other one's. Nothing is sent unless ticked (D71).
 */
export function PairDialog({
  open,
  desktop,
  onPaired,
}: {
  open: boolean;
  desktop: boolean;
  /** Called with a message once the other device is paired. */
  onPaired: (message: string) => void;
}) {
  const { t } = useTranslation();
  const libraries = useLibraries();
  const [mode, setMode] = useState<Mode>("show");
  const [selected, setSelected] = useState<string[]>([]);
  const [seeded, setSeeded] = useState(false);
  const [shown, setShown] = useState<Shown | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  // The download page's code replaces the pairing one while it is shown.
  const [gettingApp, setGettingApp] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canScan, setCanScan] = useState(false);
  // Paired: now whose it is, where this device administers a library with roles.
  const [owning, setOwning] = useState<{
    deviceId: string;
    name: string;
    owners: PairingOwners;
    message: string;
  } | null>(null);
  const paired = useRef(onPaired);
  useEffect(() => {
    paired.current = onPaired;
  }, [onPaired]);

  // Each time it opens, the active library comes ticked, as Design asks.
  const activeId = libraries.active?.id;
  if (open && !seeded && activeId) {
    setSeeded(true);
    setSelected(libraries.active?.canShare ? [activeId] : []);
    setMode("show");
  }
  if (!open && seeded) {
    setSeeded(false);
    setShown(null);
    setCode("");
    setError(null);
    setOwning(null);
  }

  /** After pairing: ask whose the device is when it matters, or finish. */
  const finish = useRef(async (deviceId: string, name: string, message: string) => {
    const owners = await pairingOwners(deviceId).catch(() => null);
    if (owners && owners.libraries.length > 0) {
      setOwning({ deviceId, name, owners, message });
    } else {
      paired.current(message);
      closeOverlay();
    }
  });

  useEffect(() => {
    void isMobile().then(setCanScan);
  }, []);

  // The code carries what is shared, so a new selection opens a new window.
  useEffect(() => {
    if (!open || mode !== "show" || !seeded) return;
    let cancelled = false;
    openPairing(selected)
      .then(async (info) => {
        const qr = await QRCode.toString(info.code, { type: "svg", margin: 1 });
        if (!cancelled) {
          setShown({
            code: info.code,
            pin: info.pin,
            qr,
            expires: Date.now() + info.expiresInSeconds * 1000,
          });
          setError(null);
        }
      })
      .catch((e: unknown) => !cancelled && setError(t(permissionErrorKey(e) ?? syncErrorKey(e))));
    return () => {
      cancelled = true;
    };
  }, [open, mode, seeded, selected, t]);

  // While the code is shown, watch for the other device pairing through it.
  const waiting = open && mode === "show" && owning === null;
  useEffect(() => {
    if (!waiting) return;
    let known: Set<string> | null = null;
    let done = false;
    const check = async () => {
      const status = await lanStatus().catch(() => null);
      if (!status || done) return;
      const ids = new Set(status.clients.map((c) => c.deviceId));
      if (known) {
        const joined = status.clients.find((c) => !known?.has(c.deviceId));
        if (joined) {
          done = true;
          void closePairing();
          void finish.current(
            joined.deviceId,
            joined.name,
            t("pair.pairedWith", { name: joined.name }),
          );
          return;
        }
      }
      known = ids;
    };
    void check();
    const timer = setInterval(() => {
      setNow(Date.now());
      void check();
    }, 2000);
    return () => {
      done = true;
      clearInterval(timer);
    };
  }, [waiting, t]);

  const close = () => {
    void closePairing();
    closeOverlay();
  };

  const pair = (text: string) => pairUsing(() => pairWith(text, selected));

  // Whichever way it paired (the code, its QR or the number), the same steps follow.
  const pairUsing = async (pairing: () => Promise<{ deviceId: string; name: string }>) => {
    setBusy(true);
    setError(null);
    try {
      const device = await pairing();
      const report = await syncDevice(device.deviceId);
      await finish.current(
        device.deviceId,
        device.name,
        [
          t("devices.pairedAndSynced", {
            name: device.name,
            received: report.received.received,
            sent: report.sent,
          }),
          ...foreignCloudNotices(report, t, t("nav.myLibrary")),
        ].join(" "),
      );
    } catch (e) {
      setError(t(permissionErrorKey(e) ?? syncErrorKey(e)));
    } finally {
      setBusy(false);
    }
  };

  const scan = () =>
    void scanQr()
      .then((scanned) => (scanned ? pair(scanned) : undefined))
      .catch((e: unknown) =>
        setError(e instanceof CameraDenied ? t("check.cameraDenied") : String(e)),
      );

  const switchTo = (next: Mode) => {
    if (next === "enter") void closePairing();
    setShown(null);
    setError(null);
    setMode(next);
  };

  const left = shown ? Math.max(0, Math.round((shown.expires - now) / 1000)) : 0;
  const time = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;

  const choose = (
    <section className="flex flex-col gap-2">
      <div>
        <h3 className="text-sm font-semibold">
          {desktop ? t("pair.step1") : t("sharing.whatGoes")}
        </h3>
        <p className="text-[13px] text-muted-foreground">{t("sharing.whatGoesHint")}</p>
      </div>
      <LibraryChecklist selected={selected} onChange={setSelected} disabled={busy} />
    </section>
  );

  const showCode = (
    <section className="flex flex-col gap-2">
      {desktop && <h3 className="text-sm font-semibold">{t("pair.step2")}</h3>}
      <p className="text-[13px] text-muted-foreground">
        {left > 0 ? t("pair.validFor", { time }) : t("pair.sameWifi")}
      </p>
      <div className="flex flex-col items-center gap-4 rounded-xl border p-4">
        {gettingApp ? (
          <GetTheApp onBack={() => setGettingApp(false)} />
        ) : shown && left > 0 ? (
          <>
            <div
              className="w-full max-w-72 rounded-xl border bg-white p-2"
              // The SVG comes from the qrcode library, built from our own pairing code.
              dangerouslySetInnerHTML={{ __html: shown.qr }}
            />
            <div className="flex w-full items-center gap-2">
              <Input readOnly value={shown.code} className="font-mono text-xs" />
              <Button
                variant="outline"
                onClick={() => void navigator.clipboard.writeText(shown.code)}
              >
                <Copy /> {t("pair.copy")}
              </Button>
            </div>
            <div className="flex w-full flex-col items-center gap-1 rounded-lg bg-muted px-3 py-2.5 text-center">
              <span className="text-[13px] text-muted-foreground">{t("pair.pinHint")}</span>
              <span
                className="font-mono text-3xl font-semibold tracking-[0.2em] tabular-nums"
                aria-label={shown.pin.split("").join(" ")}
              >
                {shown.pin.slice(0, 3)} {shown.pin.slice(3)}
              </span>
            </div>
            <GetTheAppButton onClick={() => setGettingApp(true)} />
          </>
        ) : shown ? (
          <Button variant="outline" onClick={() => setSelected([...selected])}>
            {t("pair.newCode")}
          </Button>
        ) : (
          <div className="h-44 w-44 animate-pulse rounded-xl bg-muted" />
        )}
      </div>
    </section>
  );

  const enterCode = (
    <section className="flex flex-col gap-3">
      {desktop && <h3 className="text-sm font-semibold">{t("pair.enterTitle")}</h3>}
      {canScan && (
        <Button className="h-12" disabled={busy} onClick={scan}>
          <ScanLine /> {t("pair.scan")}
        </Button>
      )}
      <ByNumber
        busy={busy}
        onPair={(device, pin) => void pairUsing(() => pairWithPin(device.address, pin, selected))}
      />
      <p className="text-[13px] text-muted-foreground">
        {t(canScan ? "pair.pasteHint" : "pair.pasteOnly")}
      </p>
      <div className="flex gap-2">
        <Input
          aria-label={t("devices.code")}
          placeholder="KOSMOTHEKA1:…"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          className="font-mono text-xs"
        />
        <Button disabled={busy || !code.trim()} onClick={() => void pair(code)}>
          {t("devices.pair")}
        </Button>
      </div>
    </section>
  );

  const tabs = (
    <div role="tablist" className="grid grid-cols-2 gap-1 rounded-xl bg-muted p-1">
      {(["show", "enter"] as const).map((tab) => (
        <button
          key={tab}
          type="button"
          role="tab"
          aria-selected={mode === tab}
          className={cn(
            "flex h-10 items-center justify-center gap-2 rounded-lg text-sm font-medium text-muted-foreground",
            mode === tab && "bg-background text-foreground shadow-sm",
          )}
          onClick={() => switchTo(tab)}
        >
          {tab === "show" ? <QrCode className="size-4" /> : <ScanLine className="size-4" />}
          {t(tab === "show" ? "pair.showMine" : "pair.enterTheirs")}
        </button>
      ))}
    </div>
  );

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={(next) => !next && close()}
      title={t("pair.title")}
      className="sm:max-w-3xl"
    >
      {owning ? (
        <OwnerStep
          deviceId={owning.deviceId}
          deviceName={owning.name}
          owners={owning.owners}
          desktop={desktop}
          onDone={(message) => {
            onPaired(`${owning.message} ${message}`);
            closeOverlay();
          }}
        />
      ) : desktop ? (
        <>
          <div className="grid grid-cols-2 gap-6">
            {choose}
            {mode === "show" ? showCode : enterCode}
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex items-center justify-between gap-4 border-t pt-4">
            <SendSummary selected={selected} />
            <Button
              variant="outline"
              className="shrink-0"
              onClick={() => switchTo(mode === "show" ? "enter" : "show")}
            >
              {mode === "show" ? <ClipboardPaste /> : <QrCode />}
              {t(mode === "show" ? "pair.pasteTheirs" : "pair.showMine")}
            </Button>
          </div>
        </>
      ) : (
        <>
          {tabs}
          {mode === "show" ? showCode : enterCode}
          {error && <p className="text-sm text-destructive">{error}</p>}
          {choose}
          <div className="border-t pt-3">
            <SendSummary selected={selected} />
          </div>
        </>
      )}
    </ResponsiveDialog>
  );
}

/**
 * Pairing by number (D88): the devices nearby with their pairing open, by name; choose one and type the six digits
 * it shows. Like a TV or screencasting: the number proves it is the screen in front of you.
 */
function ByNumber({
  busy,
  onPair,
}: {
  busy: boolean;
  onPair: (device: NearbyDevice, pin: string) => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [nearby, setNearby] = useState<NearbyDevice[] | null>(null);
  const [chosen, setChosen] = useState<NearbyDevice | null>(null);
  const [pin, setPin] = useState("");

  const search = () => {
    setNearby(null);
    setChosen(null);
    void findPairingDevices()
      .catch(() => [])
      .then((found) => {
        setNearby(found);
        if (found.length === 1) setChosen(found[0] ?? null);
      });
  };

  if (!open) {
    return (
      <Button
        variant="outline"
        className="h-12"
        disabled={busy}
        onClick={() => {
          setOpen(true);
          search();
        }}
      >
        <KeyRound /> {t("pair.byNumber")}
      </Button>
    );
  }

  return (
    <div className="flex flex-col gap-3 rounded-xl border p-3">
      <p className="text-sm font-medium">{t("pair.byNumber")}</p>
      {nearby === null ? (
        <p className="flex items-center gap-2 text-[13px] text-muted-foreground">
          <Loader2 className="size-4 animate-spin" /> {t("pair.searching")}
        </p>
      ) : nearby.length === 0 ? (
        <div className="flex flex-col items-start gap-2">
          <p className="text-[13px] text-pretty text-muted-foreground">{t("pair.noneNearby")}</p>
          <Button variant="outline" size="sm" onClick={search}>
            {t("pair.searchAgain")}
          </Button>
        </div>
      ) : (
        <div className="flex flex-col gap-1.5" role="radiogroup" aria-label={t("pair.whichDevice")}>
          <p className="text-[13px] text-muted-foreground">{t("pair.whichDevice")}</p>
          {nearby.map((device) => (
            <button
              key={device.deviceId}
              type="button"
              role="radio"
              aria-checked={chosen?.deviceId === device.deviceId}
              className={cn(
                "flex h-11 items-center gap-2.5 rounded-lg border px-3 text-left text-sm",
                chosen?.deviceId === device.deviceId && "border-primary bg-primary/5 font-medium",
              )}
              onClick={() => setChosen(device)}
            >
              <MonitorSmartphone className="size-4 shrink-0" />
              <span className="truncate">{device.name}</span>
            </button>
          ))}
        </div>
      )}
      {chosen && (
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (pin.length === 6) onPair(chosen, pin);
          }}
        >
          <Input
            autoFocus
            inputMode="numeric"
            autoComplete="one-time-code"
            maxLength={6}
            aria-label={t("pair.pinLabel")}
            placeholder="000000"
            value={pin}
            onChange={(e) => setPin(e.target.value.replace(/\D/g, "").slice(0, 6))}
            className="h-12 text-center font-mono text-2xl tracking-[0.3em]"
          />
          <Button type="submit" className="h-12" disabled={busy || pin.length !== 6}>
            {busy ? <Loader2 className="animate-spin" /> : null}
            {t("devices.pair")}
          </Button>
        </form>
      )}
    </div>
  );
}
