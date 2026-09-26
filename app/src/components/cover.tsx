import { useEffect, useState } from "react";
import { Book, BookImage, Clapperboard, Dice5, Disc3, Gamepad2, Shapes } from "lucide-react";

import { coverSrc } from "@/lib/api";
import { cn } from "@/lib/utils";

/** The kinds of work the catalog will hold (D60); each has its own placeholder icon. */
export type CoverKind = "book" | "comic" | "game" | "film" | "videogame" | "music" | "other";

const PLACEHOLDER_ICONS = {
  book: Book,
  comic: BookImage,
  game: Dice5,
  film: Clapperboard,
  videogame: Gamepad2,
  music: Disc3,
  other: Shapes,
} satisfies Record<CoverKind, unknown>;

/** A remote `url` (not yet saved) wins over a stored `file`; otherwise a placeholder. */
export function Cover({
  file,
  url,
  alt,
  kind = "book",
  className,
}: {
  file?: string | null;
  url?: string | null;
  alt: string;
  kind?: CoverKind;
  className?: string;
}) {
  const [localSrc, setLocalSrc] = useState<{ file: string; src: string } | null>(
    null,
  );
  const [broken, setBroken] = useState<string | null>(null);

  useEffect(() => {
    if (!file) return;
    let cancelled = false;
    void coverSrc(file).then((src) => !cancelled && setLocalSrc({ file, src }));
    return () => {
      cancelled = true;
    };
  }, [file]);

  const src = url ?? (localSrc && localSrc.file === file ? localSrc.src : null);
  const box = cn(
    "flex shrink-0 items-center justify-center overflow-hidden rounded-sm bg-muted",
    className,
  );

  if (!src || broken === src) {
    const Icon = PLACEHOLDER_ICONS[kind];
    return (
      <div className={cn(box, "cover-placeholder")} aria-hidden>
        <Icon className="size-2/5 text-muted-foreground" strokeWidth={1.5} />
      </div>
    );
  }
  return (
    <div className={box}>
      <img
        src={src}
        alt={alt}
        className="size-full object-cover"
        onError={() => setBroken(src)}
      />
    </div>
  );
}
