import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

export type PageTocItem = {
  id: string;
  label: string;
  level: 1 | 2 | 3 | 4 | 5 | 6;
};

type PageTocContextValue = {
  items: PageTocItem[];
  setItems: (items: PageTocItem[]) => void;
};

const PageTocContext = createContext<PageTocContextValue | null>(null);

export function PageTocProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<PageTocItem[]>([]);
  const value = useMemo(() => ({ items, setItems }), [items]);

  return <PageTocContext.Provider value={value}>{children}</PageTocContext.Provider>;
}

export function usePageToc() {
  const context = useContext(PageTocContext);

  if (!context) {
    throw new Error("usePageToc must be used within PageTocProvider");
  }

  return context;
}
