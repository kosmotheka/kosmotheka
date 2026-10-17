import { useEffect, useState } from "react";

import { coverSrc, type Kind } from "@/lib/api";
import { COVER_SHAPES, KIND_ICONS } from "@/lib/kinds";
import { cn } from "@/lib/utils";

/**
 * A remote `url` (not yet saved) wins over a stored `file`; otherwise a placeholder
 * shaped like the kind's cover (Design's `Cover.dc.html`). Callers set the height
 * through `className`; every cover gets the same 2:3 box so lists line up; images are centred inside it, never cropped.
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
    "aspect-[2/3]",
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
        className={cn("size-full object-contain", loaded !== src && "opacity-0")}
        onLoad={() => setLoaded(src)}
        onError={() => setBroken(src)}
      />
    </div>
  );
}
