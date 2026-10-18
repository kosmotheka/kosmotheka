import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Info } from "lucide-react";

/** A line that says the cloud is run by somebody else: «Esta nube no es la oficial de Kosmotheka: la lleva …». */
export function NotOfficialNote({ host, children }: { host: string; children?: ReactNode }) {
  const { t } = useTranslation();
  return (
    <p
      role="note"
      className="flex items-start gap-2 rounded-xl border border-warning/40 bg-warning-soft px-3.5 py-3 text-[13px] leading-snug text-pretty text-foreground"
    >
      <Info className="mt-0.5 size-4 shrink-0" />
      <span>
        {t("cloud.official.note", { host })} {children}
      </span>
    </p>
  );
}
