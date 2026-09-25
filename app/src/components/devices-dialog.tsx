import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import QRCode from "qrcode";
import { Copy, Link2, RefreshCw, Trash2 } from "lucide-react";

import { Field } from "@/components/book-form";
import { ConfirmButton } from "@/components/confirm-button";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  type KnownDevice,
  type LanStatus,
  closePairing,
  forgetDevice,
  lanStatus,
  openPairing,
  pairWith,
  setDeviceName,
  syncErrorKey,
  syncWith,
} from "@/lib/api";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Called after a sync brought changes, so the catalog refreshes. */
  onChanged: () => void;
}

export function DevicesDialog({ open, onOpenChange, onChanged }: Props) {
  const { t } = useTranslation();
  const [status, setStatus] = useState<LanStatus | null>(null);
  const [name, setName] = useState("");
  const [pairing, setPairing] = useState<{ code: string; qr: string } | null>(null);
  const [codeInput, setCodeInput] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = useCallback(() => {
    lanStatus()
      .then((loaded) => {
        setStatus(loaded);
        setName(loaded.deviceName);
      })
      .catch((e: unknown) => setMessage(String(e)));
  }, []);

  useEffect(() => {
    if (open) refresh();
  }, [open, refresh]);

  const handleOpenChange = (next: boolean) => {
    if (!next) {
      if (pairing) void closePairing();
      setPairing(null);
      setMessage(null);
      setCodeInput("");
    }
    onOpenChange(next);
  };

  const run = async (key: string, action: () => Promise<string | null>) => {
    setBusy(key);
    setMessage(null);
    try {
      const result = await action();
      if (result) setMessage(result);
      refresh();
    } catch (e) {
      setMessage(t(syncErrorKey(e)));
    } finally {
      setBusy(null);
    }
  };

  const showCode = () =>
    run("pairing", async () => {
      const info = await openPairing();
      const qr = await QRCode.toString(info.code, { type: "svg", margin: 1 });
      setPairing({ code: info.code, qr });
      return null;
    });

  const pair = () =>
    run("pair", async () => {
      const paired = await pairWith(codeInput);
      setCodeInput("");
      const report = await syncWith(paired.deviceId);
      onChanged();
      return t("devices.pairedAndSynced", {
        name: paired.name,
        received: report.received.received,
        sent: report.sent,
      });
    });

  const syncNow = (device: KnownDevice) =>
    run(device.deviceId, async () => {
      const report = await syncWith(device.deviceId);
      if (report.received.received > 0) onChanged();
      return t("devices.synced", {
        name: device.name,
        received: report.received.received,
        sent: report.sent,
      });
    });

  const forget = (device: KnownDevice) =>
    run(`forget-${device.deviceId}`, async () => {
      await forgetDevice(device.deviceId);
      return t("devices.forgotten", { name: device.name });
    });

  const lastSync = (device: KnownDevice) =>
    device.lastSyncAt
      ? t("devices.lastSync", { date: new Date(device.lastSyncAt).toLocaleString() })
      : t("devices.neverSynced");

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("devices.title")}</DialogTitle>
          <DialogDescription>{t("devices.description")}</DialogDescription>
        </DialogHeader>

        <section className="space-y-2">
          <Field id="device-name" label={t("devices.thisDevice")}>
            <div className="flex gap-2">
              <Input id="device-name" value={name} onChange={(e) => setName(e.target.value)} />
              <Button
                variant="outline"
                disabled={!status || name.trim() === status.deviceName}
                onClick={() => void run("name", async () => (await setDeviceName(name), null))}
              >
                {t("form.save")}
              </Button>
            </div>
          </Field>
        </section>

        <section className="space-y-3 border-t pt-4">
          <h3 className="text-sm font-medium">{t("devices.addTitle")}</h3>
          {pairing ? (
            <div className="space-y-3">
              <p className="text-sm text-muted-foreground">{t("devices.showCodeHelp")}</p>
              <div
                className="mx-auto w-48 rounded-md bg-white p-2"
                // The SVG comes from the qrcode library, built from our own pairing code.
                dangerouslySetInnerHTML={{ __html: pairing.qr }}
              />
              <div className="flex gap-2">
                <Input readOnly value={pairing.code} className="font-mono text-xs" />
                <Button
                  variant="outline"
                  size="icon"
                  aria-label={t("devices.copy")}
                  onClick={() => void navigator.clipboard.writeText(pairing.code)}
                >
                  <Copy />
                </Button>
              </div>
              <Button
                variant="ghost"
                onClick={() => {
                  void closePairing();
                  setPairing(null);
                }}
              >
                {t("devices.stopPairing")}
              </Button>
            </div>
          ) : (
            <Button variant="outline" disabled={busy !== null} onClick={() => void showCode()}>
              <Link2 /> {t("devices.showCode")}
            </Button>
          )}
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">{t("devices.enterCodeHelp")}</p>
            <div className="flex gap-2">
              <Input
                aria-label={t("devices.code")}
                placeholder="LIBRARY1:…"
                value={codeInput}
                onChange={(e) => setCodeInput(e.target.value)}
                className="font-mono text-xs"
              />
              <Button
                disabled={busy !== null || !codeInput.trim()}
                onClick={() => void pair()}
              >
                {t("devices.pair")}
              </Button>
            </div>
          </div>
        </section>

        {message && <p className="text-sm">{message}</p>}

        {status && (status.servers.length > 0 || status.clients.length > 0) && (
          <section className="space-y-2 border-t pt-4">
            <h3 className="text-sm font-medium">{t("devices.paired")}</h3>
            <ul className="space-y-2">
              {status.servers.map((device) => (
                <li
                  key={`s-${device.deviceId}`}
                  className="flex items-center justify-between gap-2 rounded-lg border p-2"
                >
                  <div className="text-sm">
                    <p className="font-medium">{device.name}</p>
                    <p className="text-muted-foreground">{lastSync(device)}</p>
                  </div>
                  <div className="flex gap-1">
                    <Button
                      size="sm"
                      disabled={busy !== null}
                      onClick={() => void syncNow(device)}
                    >
                      <RefreshCw className={busy === device.deviceId ? "animate-spin" : ""} />
                      {t("devices.syncNow")}
                    </Button>
                    <ForgetButton device={device} onForget={() => void forget(device)} />
                  </div>
                </li>
              ))}
              {status.clients.map((device) => (
                <li
                  key={`c-${device.deviceId}`}
                  className="flex items-center justify-between gap-2 rounded-lg border p-2"
                >
                  <div className="text-sm">
                    <p className="font-medium">{device.name}</p>
                    <p className="text-muted-foreground">
                      {t("devices.syncsWithThis")} · {lastSync(device)}
                    </p>
                  </div>
                  <ForgetButton device={device} onForget={() => void forget(device)} />
                </li>
              ))}
            </ul>
          </section>
        )}
      </DialogContent>
    </Dialog>
  );
}

function ForgetButton({ device, onForget }: { device: KnownDevice; onForget: () => void }) {
  const { t } = useTranslation();
  return (
    <ConfirmButton
      trigger={
        <Button variant="ghost" size="icon" aria-label={t("devices.forget")}>
          <Trash2 />
        </Button>
      }
      title={t("devices.forgetConfirm.title", { name: device.name })}
      description={t("devices.forgetConfirm.body")}
      confirmLabel={t("devices.forget")}
      onConfirm={onForget}
    />
  );
}
