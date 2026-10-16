import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Input } from "@/components/ui/input";

interface LicensePackage {
  name: string;
  version: string;
  license: string;
  repository: string | null;
  texts: number[];
}

interface TermsEntry {
  name: string;
  version: string;
  purpose: string;
  links: { label: string; url: string }[];
}

interface LicensesFile {
  terms: TermsEntry[];
  texts: { id: string; text: string }[];
  packages: LicensePackage[];
}

/** Only web addresses become links: whatever else a package says as its repository stays text. */
const isWeb = (url: string | null): url is string => url !== null && url.startsWith("https://");

/** The notices of what ships inside the app (src/licenses.json, written by `npm run licenses`). */
export function LicensesDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const [file, setFile] = useState<LicensesFile | null>(null);
  const [failed, setFailed] = useState(false);
  const [query, setQuery] = useState("");

  // Loaded the first time it opens: the file is large and most people never open it.
  useEffect(() => {
    if (!open || file) return;
    import("@/licenses.json")
      .then((module) => setFile(module.default as unknown as LicensesFile))
      .catch(() => setFailed(true));
  }, [open, file]);

  const groups = useMemo(() => {
    if (!file) return [];
    const needle = query.trim().toLowerCase();
    const byLicense = new Map<string, LicensePackage[]>();
    for (const entry of file.packages) {
      if (needle && !entry.name.toLowerCase().includes(needle)) continue;
      byLicense.set(entry.license, [...(byLicense.get(entry.license) ?? []), entry]);
    }
    return [...byLicense.entries()].sort(
      (a, b) => b[1].length - a[1].length || a[0].localeCompare(b[0]),
    );
  }, [file, query]);

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("licenses.title")}
      className="sm:max-w-2xl"
    >
      <div className="flex flex-col gap-4 text-sm">
        <p>{t("licenses.intro")}</p>
        <p className="text-muted-foreground">{t("licenses.own")}</p>
        {failed && <p className="text-destructive">{t("licenses.failed")}</p>}
        {file && file.terms.length > 0 && (
          <section className="flex flex-col gap-2 rounded-lg border bg-muted/40 p-3">
            <h3 className="font-medium">{t("licenses.terms.title")}</h3>
            <p className="text-muted-foreground">{t("licenses.terms.intro")}</p>
            <ul className="flex flex-col gap-3">
              {file.terms.map((entry) => (
                <li key={entry.name} className="flex flex-col gap-0.5">
                  <span>
                    {entry.name}
                    {entry.version && (
                      <span className="text-muted-foreground"> {entry.version}</span>
                    )}
                  </span>
                  <span className="text-muted-foreground">
                    {t(`licenses.terms.purpose.${entry.purpose}`)}
                  </span>
                  <span className="flex flex-wrap gap-x-3">
                    {entry.links.map((link) => (
                      <ExternalLink key={link.url} url={link.url}>
                        {link.label}
                      </ExternalLink>
                    ))}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        )}
        {file && (
          <>
            <Input
              type="search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("licenses.search")}
              aria-label={t("licenses.search")}
            />
            {groups.length === 0 && (
              <p className="text-muted-foreground">{t("licenses.noResults")}</p>
            )}
            <div className="flex flex-col gap-2">
              {groups.map(([license, entries]) => (
                <details
                  key={license}
                  open={query.trim() !== "" || undefined}
                  className="rounded-lg border px-3 py-2"
                >
                  <summary className="cursor-pointer font-medium">
                    {license}{" "}
                    <span className="font-normal text-muted-foreground tabular-nums">
                      · {t("licenses.count", { count: entries.length })}
                    </span>
                  </summary>
                  <ul className="mt-2 flex flex-col gap-1">
                    {entries.map((entry) => (
                      <PackageRow
                        key={`${entry.name}@${entry.version}`}
                        entry={entry}
                        texts={file.texts}
                      />
                    ))}
                  </ul>
                </details>
              ))}
            </div>
          </>
        )}
      </div>
    </ResponsiveDialog>
  );
}

function PackageRow({ entry, texts }: { entry: LicensePackage; texts: LicensesFile["texts"] }) {
  const { t } = useTranslation();
  const [shown, setShown] = useState(false);
  return (
    <li className="flex flex-col gap-1 border-t pt-1 first:border-t-0 first:pt-0">
      <div className="flex flex-wrap items-baseline gap-x-2">
        <span className="break-all">{entry.name}</span>
        {entry.version && (
          <span className="text-muted-foreground tabular-nums">{entry.version}</span>
        )}
        {isWeb(entry.repository) && (
          <ExternalLink url={entry.repository}>{t("licenses.source")}</ExternalLink>
        )}
        {entry.texts.length > 0 && (
          <button
            type="button"
            className="text-primary underline underline-offset-2"
            aria-expanded={shown}
            onClick={() => setShown(!shown)}
          >
            {shown ? t("licenses.hideText") : t("licenses.showText")}
          </button>
        )}
      </div>
      {shown &&
        entry.texts.map((index) => (
          <pre
            key={index}
            className="max-h-72 overflow-auto rounded bg-muted/50 p-2 text-xs whitespace-pre-wrap"
          >
            {texts[index]?.text}
          </pre>
        ))}
    </li>
  );
}
