import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

/** A link that opens in the system browser, as the other external links of the app do. */
export function ExternalLink({ url, children }: { url: string; children?: ReactNode }) {
  return (
    <a
      href={url}
      className="text-primary underline underline-offset-2"
      onClick={(event) => {
        event.preventDefault();
        void openUrl(url);
      }}
    >
      {children}
    </a>
  );
}
