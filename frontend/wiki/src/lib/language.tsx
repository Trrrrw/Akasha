import {
  createContext,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  defaultLanguage,
  isLanguageCode,
  type LanguageCode,
} from "@/data/languages";

const storageKey = "akasha-language";

type LanguageContextValue = {
  language: LanguageCode;
  setLanguage: (language: LanguageCode) => void;
};

const LanguageContext = createContext<LanguageContextValue | null>(null);

function readInitialLanguage() {
  const stored = localStorage.getItem(storageKey);

  if (stored && isLanguageCode(stored)) {
    return stored;
  }

  return defaultLanguage;
}

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<LanguageCode>(readInitialLanguage);

  const value = useMemo<LanguageContextValue>(
    () => ({
      language,
      setLanguage(nextLanguage) {
        localStorage.setItem(storageKey, nextLanguage);
        setLanguageState(nextLanguage);
      },
    }),
    [language],
  );

  return (
    <LanguageContext.Provider value={value}>
      {children}
    </LanguageContext.Provider>
  );
}

export function useLanguage() {
  const context = useContext(LanguageContext);

  if (!context) {
    throw new Error("useLanguage must be used within LanguageProvider");
  }

  return context;
}
