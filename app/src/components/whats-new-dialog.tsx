import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import { Switch } from "@/components/people/role-parts";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { languageOf } from "@/i18n";
import { type WhatsNew, type WhatsNewLine, setWhatsNewSwitch } from "@/lib/api";
import { guidePage } from "@/lib/guide";

/** What changed since the last version this person saw. What changes what leaves the device comes first, with its switch. */
export function WhatsNewDialog({
  open,
  news,
  onClose,
}: {
  open: boolean;
  news: WhatsNew;
  onClose: () => void;
}) {
  const { t, i18n } = useTranslation();
  const language = languageOf(i18n.language);
  const text = (line: WhatsNewLine) => line[language] ?? line.en;
  // What each switch shows now, since the setting changes right here.
  const [on, setOn] = useState<Record<string, boolean>>({});
  const [failed, setFailed] = useState(false);

  const flip = (key: string, value: boolean) => {
    setFailed(false);
    setOn((current) => ({ ...current, [key]: value }));
    setWhatsNewSwitch(key, value).catch(() => {
      setOn((current) => ({ ...current, [key]: !value }));
      setFailed(true);
    });
  };

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={(next) => !next && onClose()}
      title={t("whatsNew.title")}
      className="sm:max-w-2xl"
    >
      <div className="flex flex-col gap-4 text-sm">
        {news.switches.length > 0 && (
          <section className="flex flex-col gap-3 rounded-lg border bg-muted/40 p-3">
            <p className="text-xs font-medium text-muted-foreground">{t("whatsNew.leaves")}</p>
            {news.switches.map((line) => {
              const key = line.switch?.key ?? "";
              return (
                <div key={key} className="flex items-center gap-3">
                  <span className="min-w-0 flex-1">{text(line)}</span>
                  <Switch
                    checked={on[key] ?? line.switch?.on ?? false}
                    label={text(line)}
                    onChange={(value) => flip(key, value)}
                  />
                </div>
              );
            })}
            {failed && <p className="text-xs text-destructive">{t("whatsNew.switchFailed")}</p>}
          </section>
        )}
        {news.versions.map((notes) => (
          <section key={notes.version} className="flex flex-col gap-1.5">
            {(news.versions.length > 1 || notes.version !== "next") && (
              <h3 className="text-xs font-medium text-muted-foreground tabular-nums">
                {notes.version === "next"
                  ? t("whatsNew.next")
                  : t("whatsNew.version", { version: notes.version })}
              </h3>
            )}
            <ul className="flex list-disc flex-col gap-1.5 pl-5">
              {notes.lines.map((line) => (
                <li key={text(line)}>{text(line)}</li>
              ))}
            </ul>
          </section>
        ))}
        {/* The app shows each version in short; the guide keeps every line in full. */}
        <p className="text-muted-foreground">
          {t("whatsNew.detail")}{" "}
          <ExternalLink url={guidePage(i18n.language, "whats-new/")}>
            {t("whatsNew.detailLink")}
          </ExternalLink>
        </p>
        <Button className="self-end" onClick={onClose}>
          {t("whatsNew.close")}
        </Button>
      </div>
    </ResponsiveDialog>
  );
}
