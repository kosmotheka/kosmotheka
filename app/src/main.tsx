import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./i18n";
import "./index.css";
import { initTheme } from "./lib/theme";

initTheme();

// Preview with sample data in a plain browser: `npm run dev`, then open `/?mock`.
if (import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock")) {
  await import("./dev/mock-ipc");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
