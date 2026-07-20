import { Outlet } from "react-router";

export function AdminListPage() {
  return (
    <main className="flex min-h-0 flex-1 flex-col overflow-hidden p-4 pt-0">
      <Outlet />
    </main>
  );
}
