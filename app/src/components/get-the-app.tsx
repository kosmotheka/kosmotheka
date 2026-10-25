import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import QRCode from "qrcode";
import { ArrowLeft, Download } from "lucide-react";

import { ExternalLink } from "@/components/external-link";
import { Button } from "@/components/ui/button";
import { languageOf } from "@/i18n";

// The website's downloads page, not the store: it already says where each version is (Google Play, Windows,
// Linux), and stays right when Android leaves the closed test without a new version of the app.
const DOWNLOADS = {
  es: "https://kosmotheka.com/es/descargas.html",
  en: "https://kosmotheka.com/en/download.html",
};

/** Under the pairing code: for the other device that does not have the app yet. */
export function GetTheAppButton({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation();
  return (
    <Button variant="link" className="h-auto gap-2 p-0 text-[13px]" onClick={onClick}>
      <Download className="size-4" /> {t("pair.noApp")}
    </Button>
  );
}

/**
 * In place of the pairing code, never beside it: with two codes on screen a camera reads either. A code to the
 * download page, and back to the pairing code.
 */
export function GetTheApp({ onBack }: { onBack: () => void }) {
  const { t, i18n } = useTranslation();
  const url = DOWNLOADS[languageOf(i18n.language)];
  const [qr, setQr] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    void QRCode.toString(url, { type: "svg", margin: 1 }).then((svg) => current && setQr(svg));
    return () => {
      current = false;
    };
  }, [url]);

  return (
    <div className="flex w-full flex-col items-center gap-3 text-center text-[13px]">
      <p className="font-medium">{t("pair.noApp")}</p>
      <p className="text-pretty text-muted-foreground">{t("pair.noAppHint")}</p>
      {qr ? (
        <div
          className="w-full max-w-72 rounded-xl border bg-white p-2"
          // The SVG comes from the qrcode library, built from our own constant address.
          dangerouslySetInnerHTML={{ __html: qr }}
        />
      ) : (
        <div className="h-44 w-44 animate-pulse rounded-xl bg-muted" />
      )}
      <ExternalLink url={url}>{url.replace("https://", "")}</ExternalLink>
      <Button variant="outline" onClick={onBack}>
        <ArrowLeft /> {t("pair.backToCode")}
      </Button>
    </div>
  );
}
