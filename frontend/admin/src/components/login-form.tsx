import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useNavigate } from "react-router";
import { useState } from "react";

export function LoginForm({
  className,
  ...props
}: React.ComponentProps<"form">) {
  const navigate = useNavigate();
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    const formData = new FormData(event.currentTarget);
    const username = String(formData.get("username") ?? "");
    const password = String(formData.get("password") ?? "");

    const res = await fetch("/admin/auth/login", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        username,
        password,
      }),
    });
    if (!res.ok) {
      const data = (await res.json().catch(() => null)) as {
        message?: string;
      } | null;
      setError(data?.message ?? "登录账号失败");
      return;
    }

    const data = await res.json();
    const { access_token, refresh_token, expires_in, user } = data;
    localStorage.setItem("access_token", access_token);
    localStorage.setItem("refresh_token", refresh_token);
    localStorage.setItem("user", JSON.stringify(user));
    localStorage.setItem("expires_at", String(Date.now() + expires_in * 1000));

    navigate("/", { replace: true });
  }

  return (
    <form
      className={cn("flex flex-col gap-6", className)}
      onSubmit={handleSubmit}
      {...props}
    >
      <FieldGroup>
        <div className="flex flex-col items-center gap-1 text-center">
          <h1 className="text-2xl font-bold">登录账号</h1>
        </div>
        <Field>
          <FieldLabel htmlFor="user-name">用户名</FieldLabel>
          <Input name="username" id="user-name" type="text" required />
        </Field>
        <Field>
          <div className="flex items-center">
            <FieldLabel htmlFor="password">密码</FieldLabel>
          </div>
          <Input name="password" id="password" type="password" required />
        </Field>
        {error ? (
          <p className="text-sm text-destructive" role="alert">
            {error}
          </p>
        ) : null}
        <Field>
          <Button type="submit">登录</Button>
        </Field>
      </FieldGroup>
    </form>
  );
}
