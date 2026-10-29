import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./i18n";
import "./index.css";
import { isMobile } from "./lib/api";
import { takeOverBackButton } from "./lib/navigation";
import { initTheme } from "./lib/theme";

initTheme();

// Preview with sample data in a plain browser: `npm run dev`, then open `/?mock`.
if (import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock")) {
  await import("./dev/mock-ipc");
}

// On a phone the app answers the back button itself; elsewhere there is none to take over.
void isMobile()
  .then((mobile) => (mobile ? takeOverBackButton() : undefined))
  .catch(() => {});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
