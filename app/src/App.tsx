import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { supportedLanguages } from "@/i18n";
import { type AppInfo, getAppInfo } from "@/lib/api";

type LoadState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "error"; message: string };

export default function App() {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<LoadState>({ status: "loading" });

  useEffect(() => {
    getAppInfo()
      .then((info) => setState({ status: "ready", info }))
      .catch((error: unknown) =>
        setState({ status: "error", message: String(error) }),
      );
  }, []);

  return (
    <main className="flex min-h-screen items-center justify-center p-4">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle className="text-2xl">{t("app.title")}</CardTitle>
          <CardDescription>{t("app.subtitle")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <AppInfoView state={state} />
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">
              {t("language.label")}
            </span>
            {supportedLanguages.map((language) => (
              <Button
                key={language}
                size="sm"
                variant={
                  i18n.resolvedLanguage === language ? "default" : "outline"
                }
                onClick={() => void i18n.changeLanguage(language)}
              >
                {t(`language.${language}`)}
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>
    </main>
  );
}

function AppInfoView({ state }: { state: LoadState }) {
  const { t, i18n } = useTranslation();

  if (state.status === "loading") {
    return <p className="text-muted-foreground">{t("info.loading")}</p>;
  }
  if (state.status === "error") {
    return (
      <p className="text-destructive">
        {t("info.error", { message: state.message })}
      </p>
    );
  }

  const { info } = state;
  const firstLaunch = info.firstLaunch
    ? new Date(info.firstLaunch).toLocaleString(i18n.resolvedLanguage)
    : "—";

  return (
    <div className="space-y-3">
      <p className="font-medium">
        {t("info.launchCount", { count: info.launchCount })}
      </p>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <dt className="text-muted-foreground">{t("info.firstLaunch")}</dt>
        <dd>{firstLaunch}</dd>
        <dt className="text-muted-foreground">{t("info.sqliteVersion")}</dt>
        <dd>{info.sqliteVersion}</dd>
        <dt className="text-muted-foreground">{t("info.databasePath")}</dt>
        <dd className="break-all font-mono text-xs">{info.databasePath}</dd>
      </dl>
    </div>
  );
}
