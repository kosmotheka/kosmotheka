import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";

/** A photo of the printed number had several ISBNs (another edition's, the ebook's): the person picks theirs. */
export function IsbnChoice({ isbns, onPick }: { isbns: string[]; onPick: (isbn: string) => void }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-2.5 rounded-[14px] border p-3 text-sm">
      <p>{t("scanner.severalIsbns")}</p>
      <div className="flex flex-wrap gap-2">
        {isbns.map((isbn) => (
          <Button key={isbn} variant="outline" size="sm" onClick={() => onPick(isbn)}>
            {isbn}
          </Button>
        ))}
      </div>
    </div>
  );
}
