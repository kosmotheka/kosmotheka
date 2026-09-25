import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./i18n";
import "./index.css";

// shadcn/ui switches themes with a `dark` class; follow the OS preference.
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
const applyTheme = () =>
  document.documentElement.classList.toggle("dark", darkQuery.matches);
applyTheme();
darkQuery.addEventListener("change", applyTheme);

// Preview with sample data in a plain browser: `npm run dev`, then open `/?mock`.
if (import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock")) {
  await import("./dev/mock-ipc");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
