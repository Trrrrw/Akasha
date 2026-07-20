import { Route, Routes } from "react-router";

import BaseLayout from "@/layouts/base-layout";
import GameLayout from "@/layouts/game-layout";
import GameOverviewPage from "@/pages/game-overview";
import { GameSectionPage } from "@/pages/game-section";
import HomePage from "@/pages/home";

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<BaseLayout />}>
        <Route index element={<HomePage />} />
        <Route path="games/:game" element={<GameLayout />}>
          <Route index element={<GameOverviewPage />} />
          <Route path=":module" element={<GameSectionPage />} />
        </Route>
      </Route>
    </Routes>
  );
}
