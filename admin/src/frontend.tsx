/**
 * This file is the entry point for the React app, it sets up the root
 * element and renders the App component to the DOM.
 *
 * It is included in `src/index.html`.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Route, Routes } from "react-router";

import "./index.css";
import { SignupForm } from "@/components/signup-form";
import { LoginForm } from "@/components/login-form";
import ProtectedRoute from "@/pages/protected-route";
import DashboardPage from "@/pages/dashboard";
import AuthPage from "@/pages/auth";
import NewsPage from "@/pages/news";
import CharactersPage from "@/pages/characters";
import OverviewPage from "@/pages/overview";
import { AdminListPage } from "@/pages/admin-list-page";
import GamesPage from "@/pages/games";

const elem = document.getElementById("root")!;
const app = (
  <StrictMode>
    <BrowserRouter basename="/admin">
      <Routes>
        <Route element={<AuthPage />}>
          <Route path="/setup" element={<SignupForm />} />
          <Route path="/login" element={<LoginForm />} />
        </Route>
        <Route element={<ProtectedRoute />}>
          <Route element={<DashboardPage />}>
            <Route index element={<OverviewPage />} />
            <Route element={<AdminListPage />}>
              <Route path="/games" element={<GamesPage />} />
              <Route path="/news" element={<NewsPage />} />
              <Route path="/characters" element={<CharactersPage />} />
            </Route>
          </Route>
        </Route>
      </Routes>
    </BrowserRouter>
  </StrictMode>
);

if (import.meta.hot) {
  // With hot module reloading, `import.meta.hot.data` is persisted.
  const root = (import.meta.hot.data.root ??= createRoot(elem));
  root.render(app);
} else {
  // The hot module reloading API is not available in production.
  createRoot(elem).render(app);
}
