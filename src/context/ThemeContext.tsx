import React, { createContext, useContext, useEffect, useState } from "react";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = "light" | "dark";

interface ThemeContextType {
  themePref: ThemePreference;
  resolvedTheme: ResolvedTheme;
  setThemePref: (pref: ThemePreference) => void;
}

const ThemeContext = createContext<ThemeContextType>({
  themePref: "light",
  resolvedTheme: "light",
  setThemePref: () => {},
});

const THEME_STORAGE_KEY = "friedchip_theme_pref";

const getInitialThemePref = (): ThemePreference => {
  if (typeof window !== "undefined") {
    try {
      const hashQuery = window.location.hash.includes("?")
        ? window.location.hash.split("?")[1]
        : "";
      const urlParams = new URLSearchParams(window.location.search || hashQuery);
      const urlTheme = urlParams.get("theme") as ThemePreference;
      if (urlTheme === "light" || urlTheme === "dark" || urlTheme === "system") {
        return urlTheme;
      }
    } catch {
      // Fallback
    }
  }

  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY) as ThemePreference;
    if (stored === "light" || stored === "dark" || stored === "system") {
      return stored;
    }
  } catch {
    // Fallback
  }
  // Light mode is default on first launch per project requirements
  return "light";
};

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [themePref, setThemePrefState] = useState<ThemePreference>(getInitialThemePref);

  const [systemIsDark, setSystemIsDark] = useState<boolean>(() => {
    if (typeof window !== "undefined" && window.matchMedia) {
      return window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    return false;
  });

  // Track system OS appearance changes
  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = (e: MediaQueryListEvent) => {
      setSystemIsDark(e.matches);
    };

    if (mediaQuery.addEventListener) {
      mediaQuery.addEventListener("change", handleChange);
      return () => mediaQuery.removeEventListener("change", handleChange);
    } else if ("addListener" in mediaQuery) {
      (mediaQuery as { addListener: (cb: (e: MediaQueryListEvent) => void) => void }).addListener(handleChange);
      return () => (mediaQuery as { removeListener: (cb: (e: MediaQueryListEvent) => void) => void }).removeListener(handleChange);
    }
  }, []);

  const resolvedTheme: ResolvedTheme =
    themePref === "system" ? (systemIsDark ? "dark" : "light") : themePref;

  // Apply data-theme to <html>
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", resolvedTheme);
    document.documentElement.setAttribute("data-theme-pref", themePref);
    document.documentElement.style.colorScheme = resolvedTheme;
  }, [resolvedTheme, themePref]);

  const setThemePref = (pref: ThemePreference) => {
    setThemePrefState(pref);
    try {
      localStorage.setItem(THEME_STORAGE_KEY, pref);
    } catch {
      // Storage unavailable
    }
  };

  return (
    <ThemeContext.Provider value={{ themePref, resolvedTheme, setThemePref }}>
      {children}
    </ThemeContext.Provider>
  );
};

export const useTheme = () => useContext(ThemeContext);
