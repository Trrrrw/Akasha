import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { useNavigate } from "react-router";
import { useState } from "react";
import { useOutletContext } from "react-router";

import type { AuthPageOutletContext } from "@/pages/auth";

export function SignupForm({
  className,
  ...props
}: React.ComponentProps<"form">) {
  const navigate = useNavigate();
  const { refreshSetupStatus } = useOutletContext<AuthPageOutletContext>();
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: React.SubmitEvent<HTMLFormElement>) {
    event.preventDefault();

    const formData = new FormData(event.currentTarget);
    const username = String(formData.get("username") ?? "");
    const setupToken = String(formData.get("setupToken") ?? "");
    const password = String(formData.get("password") ?? "");
    const confirmPassword = String(formData.get("confirmPassword") ?? "");

    if (password !== confirmPassword) {
      setError("两次输入的密码不一致");
      return;
    }

    const res = await fetch("/admin/auth/setup", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${setupToken}`,
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
      setError(data?.message ?? "创建账号失败");
      return;
    }

    await refreshSetupStatus();
    navigate("/login", { replace: true });
  }

  return (
    <form
      className={cn("flex flex-col gap-6", className)}
      onSubmit={handleSubmit}
      {...props}
    >
      <FieldGroup>
        <div className="flex flex-col items-center gap-1 text-center">
          <h1 className="text-2xl font-bold">初始化账号</h1>
        </div>
        <Field>
          <FieldLabel htmlFor="user-name">用户名</FieldLabel>
          <Input name="username" id="user-name" type="text" required />
        </Field>
        <Field>
          <FieldLabel htmlFor="setup-token">Setup Token</FieldLabel>
          <Input name="setupToken" id="setup-token" type="text" required />
          <FieldDescription>服务器日志中输出的随机 Token</FieldDescription>
        </Field>
        <Field>
          <FieldLabel htmlFor="password">密码</FieldLabel>
          <Input name="password" id="password" type="password" required />
          <FieldDescription>至少 8 个字符</FieldDescription>
        </Field>
        <Field>
          <FieldLabel htmlFor="confirm-password">确认密码</FieldLabel>
          <Input
            name="confirmPassword"
            id="confirm-password"
            type="password"
            required
          />
        </Field>
        <Field>
          <Button type="submit">创建账号</Button>
        </Field>
      </FieldGroup>
    </form>
  );
}
