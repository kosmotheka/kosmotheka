import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import QRCode from "qrcode";
import { ClipboardPaste, Copy, QrCode, ScanLine } from "lucide-react";

import { LibraryChecklist, SendSummary } from "@/components/libraries/library-checklist";
import { useLibraries } from "@/components/libraries-context";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { closePairing, isMobile, lanStatus, openPairing, pairWith, syncErrorKey } from "@/lib/api";
import { closeOverlay } from "@/lib/navigation";
import { CameraDenied, scanQr } from "@/lib/scanner";
import { syncDevice } from "@/lib/sync-errors";
import { cn } from "@/lib/utils";

type Mode = "show" | "enter";

interface Shown {
  code: string;
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
  const [error, setError] = useState<string | null>(null);
  const [canScan, setCanScan] = useState(false);
  const paired = useRef(onPaired);
  useEffect(() => {
    paired.current = onPaired;
  }, [onPaired]);

  // Each time it opens, the active library comes ticked, as Design asks.
  const activeId = libraries.active?.id;
  if (open && !seeded && activeId) {
    setSeeded(true);
    setSelected([activeId]);
    setMode("show");
  }
  if (!open && seeded) {
    setSeeded(false);
    setShown(null);
    setCode("");
    setError(null);
  }

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
          setShown({ code: info.code, qr, expires: Date.now() + info.expiresInSeconds * 1000 });
          setError(null);
        }
      })
      .catch((e: unknown) => !cancelled && setError(t(syncErrorKey(e))));
    return () => {
      cancelled = true;
    };
  }, [open, mode, seeded, selected, t]);

  // While the code is shown, watch for the other device pairing through it.
  useEffect(() => {
    if (!open || mode !== "show") return;
    let known: Set<string> | null = null;
    const check = async () => {
      const status = await lanStatus().catch(() => null);
      if (!status) return;
      const ids = new Set(status.clients.map((c) => c.deviceId));
      if (known) {
        const joined = status.clients.find((c) => !known?.has(c.deviceId));
        if (joined) {
          void closePairing();
          paired.current(t("pair.pairedWith", { name: joined.name }));
          closeOverlay();
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
    return () => clearInterval(timer);
  }, [open, mode, t]);

  const close = () => {
    void closePairing();
    closeOverlay();
  };

  const pair = async (text: string) => {
    setBusy(true);
    setError(null);
    try {
      const device = await pairWith(text, selected);
      const report = await syncDevice(device.deviceId);
      onPaired(
        t("devices.pairedAndSynced", {
          name: device.name,
          received: report.received.received,
          sent: report.sent,
        }),
      );
      closeOverlay();
    } catch (e) {
      setError(t(syncErrorKey(e)));
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
        {shown && left > 0 ? (
          <>
            <div
              className="w-44 rounded-xl border bg-white p-2"
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
      <p className="text-[13px] text-muted-foreground">{t(canScan ? "pair.pasteHint" : "pair.pasteOnly")}</p>
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
      {desktop ? (
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
