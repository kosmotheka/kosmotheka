// Choosing a work's cover in its form: a photo or an image of one's own, or another one from the catalogues, searched
// again with what the form says now (an ISBN added, a title corrected).

import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Camera, Images, Loader2, Search } from "lucide-react";

import { Cover } from "@/components/cover";
import { SheetOrDialog } from "@/components/shell/sheet-or-dialog";
import { useDesktop } from "@/components/shell/use-desktop";
import { Button } from "@/components/ui/button";
import { languageOf } from "@/i18n";
import { type Kind, type Source, lookupIsbn, searchWorks } from "@/lib/api";
import { takePhoto } from "@/lib/photo";

/** A cover found in a catalogue, with where it came from. */
interface Found {
  url: string;
  title: string;
  source: Source | null;
}

// Enough to choose from without a long scroll.
const MAX_FOUND = 12;

export function CoverChooser({
  open,
  onClose,
  phone,
  kind,
  isbn,
  title,
  author,
  onOwn,
  onFound,
}: {
  open: boolean;
  onClose: () => void;
  /** The camera only on a phone. */
  phone: boolean;
  kind: Kind;
  isbn: string;
  title: string;
  author: string | null;
  /** A photo or an image, in base64, taken in the tap itself (see `takePhoto`). */
  onOwn: (taking: Promise<string | null>) => void;
  onFound: (url: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const desktop = useDesktop();
  const [found, setFound] = useState<Found[] | null>(null);
  const [searching, setSearching] = useState(false);
  // Bumped on close: a search still running when the sheet closed does not fill it when it opens again.
  const run = useRef(0);

  const close = () => {
    run.current++;
    setFound(null);
    setSearching(false);
    onClose();
  };

  const own = (camera: boolean) => {
    onOwn(takePhoto({ camera }));
    close();
  };

  // The ISBN first, the most exact; then the title (and author), as typed now.
  const search = async () => {
    const id = ++run.current;
    setSearching(true);
    const covers: Found[] = [];
    const add = (url: string | null, workTitle: string, source: Source | null) => {
      if (url && !covers.some((c) => c.url === url)) covers.push({ url, title: workTitle, source });
    };
    try {
      if (isbn.trim()) {
        const byIsbn = await lookupIsbn(isbn.trim()).catch(() => null);
        if (byIsbn?.found) add(byIsbn.coverUrl, byIsbn.title ?? title, byIsbn.sources[0] ?? null);
      }
      const query = [title.trim(), author?.trim()].filter(Boolean).join(" ");
      if (query.length >= 2) {
        const byTitle = await searchWorks(kind, query, languageOf(i18n.language)).catch(() => null);
        for (const candidate of byTitle?.candidates ?? []) {
          add(candidate.coverUrl, candidate.title, candidate.source);
        }
      }
      if (id === run.current) setFound(covers.slice(0, MAX_FOUND));
    } finally {
      if (id === run.current) setSearching(false);
    }
  };

  const big = "h-12 w-full justify-start gap-3 rounded-xl text-[15px]";

  return (
    <SheetOrDialog
      open={open}
      desktop={desktop}
      onClose={close}
      title={t("form.cover.title")}
      description={t("form.cover.title")}
    >
      <div className="flex flex-col gap-2">
        {phone && (
          <Button variant="outline" className={big} disabled={searching} onClick={() => own(true)}>
            <Camera className="size-5" /> {t("form.cover.photo")}
          </Button>
        )}
        <Button variant="outline" className={big} disabled={searching} onClick={() => own(false)}>
          <Images className="size-5" /> {t("form.cover.image")}
        </Button>
        <Button
          variant="outline"
          className={big}
          disabled={searching}
          onClick={() => void search()}
        >
          {searching ? <Loader2 className="size-5 animate-spin" /> : <Search className="size-5" />}
          {t("form.cover.search")}
        </Button>
        <p className="text-[13px] text-pretty text-muted-foreground">
          {t("form.cover.searchHint")}
        </p>
      </div>

      {found && found.length === 0 && (
        <p role="status" className="rounded-[14px] border p-3 text-sm">
          {t("form.cover.noneFound")}
        </p>
      )}
      {found && found.length > 0 && (
        <ul className="grid max-h-[45vh] grid-cols-3 gap-3 overflow-y-auto sm:grid-cols-4">
          {found.map((cover) => (
            <li key={cover.url}>
              <button
                type="button"
                className="flex w-full flex-col items-center gap-1 rounded-lg p-1 text-center hover:bg-muted"
                onClick={() => {
                  onFound(cover.url);
                  close();
                }}
              >
                <Cover url={cover.url} alt={cover.title} kind={kind} className="h-28" />
                <span className="line-clamp-2 text-xs">{cover.title}</span>
                {cover.source && (
                  <span className="text-[11px] text-muted-foreground">
                    {t(`lookup.sources.${cover.source}`)}
                  </span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </SheetOrDialog>
  );
}
