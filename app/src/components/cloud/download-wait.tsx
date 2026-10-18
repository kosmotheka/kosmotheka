import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { useAccount } from "@/lib/account";
import { useCloud } from "@/lib/cloud";
import { hostOf } from "@/lib/server";

// A sync that finishes at once is not worth a screen; one that lasts is a library coming down.
const SHOW_AFTER_MS = 2500;

/**
 * «Downloading your libraries from the cloud» (Design `Download`, D76): only while the first sync of this
 * device with nothing linked is still running. There is no progress to show: `sync_all` reports at the end.
 */
export function DownloadWait() {
  const { t } = useTranslation();
  const cloud = useCloud();
  const account = useAccount();
  const waiting = cloud.syncing && !cloud.status.some((s) => s.linkedHere);
  const [slow, setSlow] = useState(false);

  useEffect(() => {
    if (!waiting) return;
    const timer = window.setTimeout(() => setSlow(true), SHOW_AFTER_MS);
    return () => {
      window.clearTimeout(timer);
      setSlow(false);
    };
  }, [waiting]);

  if (!waiting || !slow) return null;
  return (
    <Alert className="mb-4">
      <Loader2 className="animate-spin" />
      <AlertTitle>{t("cloud.download.title")}</AlertTitle>
      <AlertDescription>
        {t("cloud.download.lead")}
        {!account.official &&
          ` ${t("cloud.official.downloading", { host: hostOf(account.server) })}`}
      </AlertDescription>
    </Alert>
  );
}
