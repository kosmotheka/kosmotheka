import { save } from "@tauri-apps/plugin-dialog";
import { Download } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { ExternalLink } from "@/components/external-link";
import { ResponsiveDialog } from "@/components/shell/responsive-dialog";
import { Button } from "@/components/ui/button";
import { exportLicenses } from "@/lib/api";

interface LicensePackage {
  name: string;
  version: string;
  license: string;
}

interface TermsEntry {
  name: string;
  version: string;
  purpose: string;
  links: { label: string; url: string }[];
}

interface LicensesFile {
  terms: TermsEntry[];
  packages: LicensePackage[];
}

/** The names worth saying out loud. A name that is no longer in the file is left out of the screen. */
const FEATURED = [
  { name: "tauri", label: "Tauri" },
  { name: "react", label: "React" },
  { name: "SQLite (inside libsqlite3-sys)", label: "SQLite" },
  { name: "radix-ui", label: "Radix UI" },
];

/** Licence id → the key of its plain-language line; any other id gets the generic one. */
const KINDS: Record<string, string> = {
  MIT: "mit",
  "Apache-2.0": "apache",
  "BSD-3-Clause": "bsd",
  ISC: "isc",
  Zlib: "zlib",
  "Unicode-3.0": "unicode",
  "MPL-2.0": "mpl",
  Unlicense: "publicDomain",
  "CC0-1.0": "publicDomain",
  "Public domain": "publicDomain",
};

/** The licences inside an SPDX expression: `Apache-2.0 OR MIT` has two; an exception (`WITH …`) is not one. */
const idsOf = (expression: string): string[] =>
  expression
    .split(/[()]|\s+(?:OR|AND)\s+/)
    .map((id) => id.replace(/\s+WITH\s+.*/, "").trim())
    .filter(Boolean);

/** A summary of what ships inside the app (src/licenses.json); the full list is saved as a text file by Rust. */
export function LicensesDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, i18n } = useTranslation();
  const [file, setFile] = useState<LicensesFile | null>(null);
  const [failed, setFailed] = useState(false);
  const [saving, setSaving] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);

  // Loaded the first time it opens: the file is large and most people never open it.
  useEffect(() => {
    if (!open || file) return;
    import("@/licenses.json")
      .then((module) => setFile(module.default as unknown as LicensesFile))
      .catch(() => setFailed(true));
  }, [open, file]);

  const kinds = useMemo(() => {
    const counts = new Map<string, number>();
    for (const entry of file?.packages ?? []) {
      for (const id of new Set(idsOf(entry.license))) counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    return [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  }, [file]);

  const featured = FEATURED.flatMap(({ name, label }) => {
    const entry = file?.packages.find((p) => p.name === name);
    return entry ? [{ label, license: entry.license }] : [];
  });

  const saveList = async () => {
    setResult(null);
    const english = i18n.language.startsWith("en");
    const path = await save({
      defaultPath: english ? "kosmotheka-licenses.txt" : "kosmotheka-licencias.txt",
      filters: [{ name: t("licenses.fileType"), extensions: ["txt"] }],
    });
    if (!path) return;
    setSaving(true);
    try {
      await exportLicenses(path);
      setResult({ ok: true, message: t("licenses.saved") });
    } catch (e) {
      setResult({ ok: false, message: `${t("licenses.saveFailed")} ${String(e)}` });
    } finally {
      setSaving(false);
    }
  };

  return (
    <ResponsiveDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t("licenses.title")}
      className="sm:max-w-2xl"
    >
      <div className="flex flex-col gap-4 text-sm">
        {file && <p>{t("licenses.summary", { count: file.packages.length })}</p>}
        <p className="text-muted-foreground">{t("licenses.own")}</p>
        {failed && <p className="text-destructive">{t("licenses.failed")}</p>}
        {featured.length > 0 && (
          <section className="flex flex-col gap-1">
            <h3 className="font-medium">{t("licenses.featured")}</h3>
            <ul className="flex flex-col gap-0.5">
              {featured.map((entry) => (
                <li key={entry.label}>
                  {entry.label} <span className="text-muted-foreground">· {entry.license}</span>
                </li>
              ))}
            </ul>
          </section>
        )}
        {file && (
          <section className="flex flex-col gap-2">
            <h3 className="font-medium">{t("licenses.kindsTitle")}</h3>
            <p className="text-muted-foreground">{t("licenses.kindsIntro")}</p>
            <ul className="flex flex-col gap-2">
              {kinds.map(([id, count]) => (
                <li key={id} className="flex flex-col gap-0.5">
                  <span className="font-medium">
                    {id}{" "}
                    <span className="font-normal text-muted-foreground tabular-nums">
                      · {t("licenses.count", { count })}
                    </span>
                  </span>
                  <span className="text-muted-foreground">
                    {t(`licenses.kinds.${KINDS[id] ?? "other"}`)}
                  </span>
                </li>
              ))}
            </ul>
          </section>
        )}
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
        <section className="flex flex-col gap-2">
          <p className="text-muted-foreground">{t("licenses.fullList")}</p>
          <div>
            <Button
              type="button"
              variant="outline"
              disabled={saving}
              onClick={() => void saveList()}
            >
              <Download /> {t("licenses.save")}
            </Button>
          </div>
          {result && (
            <p role="status" className={result.ok ? undefined : "text-destructive"}>
              {result.message}
            </p>
          )}
        </section>
        <Button className="self-end" onClick={() => onOpenChange(false)}>
          {t("whatsNew.close")}
        </Button>
      </div>
    </ResponsiveDialog>
  );
}
