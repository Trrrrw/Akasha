import { useEffect, useState } from "react";
import { Navigate } from "react-router";
import { Outlet } from "react-router";

type SetupStatus = {
  initialized: boolean;
};

type TokenResponse = {
  access_token: string;
  refresh_token: string;
  expires_in: number;
};

type ProtectedRouteState = "checking" | "setup" | "login" | "authorized";

export default function ProtectedRoute() {
  const [state, setState] = useState<ProtectedRouteState>("checking");

  useEffect(() => {
    async function bootstrap() {
      const setupRes = await fetch("/admin/auth/setup-status");

      if (!setupRes.ok) {
        setState("setup");
        return;
      }

      const setup = (await setupRes.json()) as SetupStatus;

      if (!setup.initialized) {
        setState("setup");
        return;
      }

      const accessToken = localStorage.getItem("access_token");
      const refreshToken = localStorage.getItem("refresh_token");

      if (!accessToken || !refreshToken) {
        clearTokens();
        setState("login");
        return;
      }

      const meRes = await fetch("/admin/auth/me", {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      });

      if (meRes.ok) {
        setState("authorized");
        return;
      }

      const refreshRes = await fetch("/admin/auth/refresh", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          refresh_token: refreshToken,
        }),
      });

      if (!refreshRes.ok) {
        clearTokens();
        setState("login");
        return;
      }

      const tokens = (await refreshRes.json()) as TokenResponse;

      localStorage.setItem("access_token", tokens.access_token);
      localStorage.setItem("refresh_token", tokens.refresh_token);

      const meAfterRefreshRes = await fetch("/admin/auth/me", {
        headers: {
          Authorization: `Bearer ${tokens.access_token}`,
        },
      });

      if (!meAfterRefreshRes.ok) {
        clearTokens();
        setState("login");
        return;
      }

      setState("authorized");
    }

    bootstrap().catch(() => {
      clearTokens();
      setState("login");
    });
  }, []);

  if (state === "checking") {
    return null;
  }

  if (state === "setup") {
    return <Navigate to="/setup" replace />;
  }

  if (state === "login") {
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
}

function clearTokens() {
  localStorage.removeItem("access_token");
  localStorage.removeItem("refresh_token");
}
