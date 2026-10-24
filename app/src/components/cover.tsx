import { useEffect, useState, useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";

import { coverSrc, type Kind } from "@/lib/api";
import { COVER_SHAPES, KIND_ICONS } from "@/lib/kinds";
import { cn } from "@/lib/utils";

// Covers that come from another device after the book did (D82): a cover whose file was missing is tried again
// whenever some arrive. One listener for every cover on screen.
let arrivals = 0;
const watchers = new Set<() => void>();
let listening = false;
function watchArrivals(onChange: () => void) {
  if (!listening) {
    listening = true;
    const arrived = () => {
      arrivals += 1;
      watchers.forEach((watcher) => watcher());
    };
    void listen("covers-arrived", arrived).catch(() => {});
    void listen("library-changed", arrived).catch(() => {});
  }
  watchers.add(onChange);
  return () => watchers.delete(onChange);
}

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
  const arrived = useSyncExternalStore(watchArrivals, () => arrivals);

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

  if (!src || broken === `${arrived}:${src}`) {
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
        // Only an image that failed before is made again, so that it is fetched again.
        key={broken === null ? 0 : arrived}
        src={src}
        alt={alt}
        className={cn("size-full object-contain", loaded !== src && "opacity-0")}
        onLoad={() => setLoaded(src)}
        onError={() => setBroken(`${arrived}:${src}`)}
      />
    </div>
  );
}
