import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { Keyboard, X } from "lucide-react";

import { cancelScan, typeInstead, useScanTarget } from "@/lib/scanner";

/**
 * Drawn over the camera while the barcode plugin scans windowed (see `lib/scanner.ts`).
 * Everything else is hidden by the `scanning` class, so the camera shows through.
 */
export function ScannerOverlay() {
  const { t } = useTranslation();
  const target = useScanTarget();
  if (!target) return null;

  const frame = target === "qr" ? "aspect-square w-[68vw]" : "aspect-[3/2] w-[80vw]";
  return createPortal(
    <div
      id="scanner-overlay"
      className="fixed inset-0 z-[200] flex flex-col items-center justify-center text-white"
    >
      <button
        type="button"
        onClick={() => void cancelScan()}
        aria-label={t("scanner.close")}
        className="absolute top-[calc(env(safe-area-inset-top)+12px)] left-3 flex size-12 items-center justify-center rounded-full bg-black/50"
      >
        <X className="size-6" />
      </button>
      <div
        className={`${frame} max-w-[420px] rounded-3xl border-[3px] border-white/90 shadow-[0_0_0_100vmax_rgb(0_0_0/45%)]`}
      />
      <p className="mt-8 max-w-[80vw] rounded-2xl bg-black/50 px-4 py-2 text-center text-sm font-medium">
        {t(target === "qr" ? "scanner.qrHint" : "scanner.barcodeHint")}
      </p>
      {target === "barcode" && (
        <button
          type="button"
          onClick={() => void typeInstead()}
          className="absolute bottom-[calc(env(safe-area-inset-bottom)+24px)] flex items-center gap-2 rounded-full bg-black/50 px-5 py-3 text-sm font-medium"
        >
          <Keyboard className="size-5" /> {t("scanner.type")}
        </button>
      )}
    </div>,
    document.body,
  );
}
