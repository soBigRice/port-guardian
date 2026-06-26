import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from "react";
import { zh, en } from "./locales";

// ---------- Types ----------

export type Language = "zh" | "en";

/** Recursively extract all dot-separated key paths from an object type */
type NestedKeyOf<T> = T extends object
  ? {
      [K in keyof T]: K extends string
        ? T[K] extends object
          ? `${K}` | `${K}.${NestedKeyOf<T[K]>}`
          : `${K}`
        : never;
    }[keyof T]
  : never;

export type TranslationKey = NestedKeyOf<typeof zh>;

/** A translations object where all leaves are `string` (not literal) */
type Translations = typeof zh;

// ---------- Translations registry ----------

const translations: Record<Language, Translations> = { zh, en };

// ---------- Helpers ----------

function getNestedValue(obj: unknown, path: string): string | undefined {
  let current: unknown = obj;
  for (const part of path.split(".")) {
    if (current === null || typeof current !== "object") return undefined;
    current = (current as Record<string, unknown>)[part];
  }
  return typeof current === "string" ? current : undefined;
}

function makeT(language: Language) {
  return function t(key: TranslationKey, params?: Record<string, string | number>): string {
    const value = getNestedValue(translations[language], key);
    if (value === undefined) return key;
    if (!params) return value;
    return Object.entries(params).reduce(
      (str, [k, v]) => str.replace(new RegExp(`\\{${k}\\}`, "g"), String(v)),
      value,
    );
  };
}

// ---------- Context ----------

interface LanguageContextValue {
  language: Language;
  setLanguage: (lang: Language) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}

const LanguageContext = createContext<LanguageContextValue | null>(null);

// ---------- Provider ----------

function getInitialLanguage(): Language {
  const saved = localStorage.getItem("pg-language");
  if (saved === "zh" || saved === "en") return saved;
  return "zh";
}

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(getInitialLanguage);
  const [t, setT] = useState(() => makeT(language));

  const setLanguage = useCallback((lang: Language) => {
    setLanguageState(lang);
    localStorage.setItem("pg-language", lang);
  }, []);

  useEffect(() => {
    setT(() => makeT(language));
    document.documentElement.lang = language;
  }, [language]);

  return (
    <LanguageContext.Provider value={{ language, setLanguage, t }}>
      {children}
    </LanguageContext.Provider>
  );
}

// ---------- Hook ----------

export function useTranslation() {
  const ctx = useContext(LanguageContext);
  if (!ctx) throw new Error("useTranslation must be used within LanguageProvider");
  return ctx;
}
