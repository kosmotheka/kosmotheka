import { useEffect, useState } from "react";
import { BookImage } from "lucide-react";

import { coverSrc } from "@/lib/api";
import { cn } from "@/lib/utils";

/** A remote `url` (not yet saved) wins over a stored `file`; otherwise a placeholder. */
export function Cover({
  file,
  url,
  alt,
  className,
}: {
  file?: string | null;
  url?: string | null;
  alt: string;
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
    return (
      <div className={box} aria-hidden>
        <BookImage className="size-1/2 text-muted-foreground" />
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
