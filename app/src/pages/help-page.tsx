import { useTranslation } from "react-i18next";
import { CircleHelp } from "lucide-react";

export function HelpPage() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center gap-3 py-16 text-center">
      <span className="flex size-13 items-center justify-center rounded-full bg-muted text-muted-foreground">
        <CircleHelp className="size-6" />
      </span>
      <h1 className="text-lg font-semibold">{t("help.title")}</h1>
      <p className="max-w-md text-sm text-pretty text-muted-foreground">{t("help.soon")}</p>
    </div>
  );
}
