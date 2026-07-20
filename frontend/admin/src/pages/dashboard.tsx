import { AppSidebar } from "@/components/app-sidebar";
import { SiteHeader } from "@/components/site-header";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { useEffect, useState } from "react";
import { Outlet } from "react-router";

type MeResponse = {
  username: string;
};

type DashboardUser = {
  name: string;
  email: string;
  avatar: string;
};

export default function DashboardPage() {
  const [user, setUser] = useState<DashboardUser>({
    name: "Admin",
    email: "管理员",
    avatar: "",
  });

  useEffect(() => {
    async function loadMe() {
      const accessToken = localStorage.getItem("access_token");

      if (!accessToken) {
        return;
      }

      const res = await fetch("/admin/auth/me", {
        headers: {
          Authorization: `Bearer ${accessToken}`,
        },
      });

      if (!res.ok) {
        return;
      }

      const data = (await res.json()) as MeResponse;

      setUser({
        name: data.username,
        email: "管理员",
        avatar: "",
      });
    }

    loadMe().catch(() => null);
  }, []);

  return (
    <SidebarProvider>
      <AppSidebar user={user} />
      <SidebarInset className="h-svh overflow-hidden md:h-[calc(100svh-1rem)]">
        <SiteHeader />
        <Outlet />
      </SidebarInset>
    </SidebarProvider>
  );
}
