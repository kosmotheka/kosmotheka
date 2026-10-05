import { useEffect, useState } from "react";

import { coverSrc, type Kind } from "@/lib/api";
import { COVER_SHAPES, KIND_ICONS, type CoverShape } from "@/lib/kinds";
import { cn } from "@/lib/utils";

const SHAPE_ASPECT: Record<CoverShape, string> = {
  tall: "aspect-[2/3]",
  square: "aspect-square",
  case: "aspect-[5/7]",
  game: "aspect-[3/4]",
};

/**
 * A remote `url` (not yet saved) wins over a stored `file`; otherwise a placeholder
 * shaped like the kind's cover (Design's `Cover.dc.html`). Callers set the height
 * through `className`; the width follows the kind's aspect ratio.
 */
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
  kind?: Kind;
  className?: string;
}) {
  const [localSrc, setLocalSrc] = useState<{ file: string; src: string } | null>(null);
  const [broken, setBroken] = useState<string | null>(null);
  // Remote covers can take a while: the box pulses until the image has arrived.
  const [loaded, setLoaded] = useState<string | null>(null);

  useEffect(() => {
    if (!file) return;
    let cancelled = false;
    void coverSrc(file).then((src) => !cancelled && setLocalSrc({ file, src }));
    return () => {
      cancelled = true;
    };
  }, [file]);

  const src = url ?? (localSrc && localSrc.file === file ? localSrc.src : null);
  const shape = COVER_SHAPES[kind];
  const box = cn(
    "relative flex shrink-0 items-center justify-center overflow-hidden rounded-sm bg-muted",
    SHAPE_ASPECT[shape],
    className,
  );

  if (!src || broken === src) {
    const Icon = KIND_ICONS[kind];
    return (
      <div className={cn(box, "cover-placeholder")} aria-hidden>
        {shape === "case" && <span className="cover-spine" />}
        {shape === "game" && <span className="cover-band" />}
        <Icon className="size-2/5 text-muted-foreground" strokeWidth={1.5} />
      </div>
    );
  }
  return (
    <div className={cn(box, loaded !== src && "animate-pulse")}>
      <img
        src={src}
        alt={alt}
        className={cn("size-full object-cover", loaded !== src && "opacity-0")}
        onLoad={() => setLoaded(src)}
        onError={() => setBroken(src)}
      />
    </div>
  );
}
