import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Immediate synchronous theme application to prevent any visual flash
try {
  const hashQuery = window.location.hash.includes("?")
    ? window.location.hash.split("?")[1]
    : "";
  const urlParams = new URLSearchParams(window.location.search || hashQuery);
  const urlTheme = urlParams.get("theme");
  const pref =
    urlTheme === "light" || urlTheme === "dark" || urlTheme === "system"
      ? urlTheme
      : localStorage.getItem("friedchip_theme_pref") || "light";

  const isDark =
    pref === "dark" ||
    (pref === "system" && window.matchMedia?.("(prefers-color-scheme: dark)").matches);
  const resolved = isDark ? "dark" : "light";
  document.documentElement.setAttribute("data-theme", resolved);
  document.documentElement.setAttribute("data-theme-pref", pref);
  document.documentElement.style.colorScheme = resolved;
} catch {
  // Fallback to light mode
  document.documentElement.setAttribute("data-theme", "light");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
