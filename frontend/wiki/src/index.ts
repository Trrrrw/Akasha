import { serve } from "bun";
import index from "./index.html";

const BACKEND_ORIGIN = "http://127.0.0.1:7040";

/** 将开发环境的 API 请求转发到本地后端 */
function proxyApi(request: Request): Promise<Response> {
  const requestUrl = new URL(request.url);
  const backendUrl = new URL(
    `${requestUrl.pathname}${requestUrl.search}`,
    BACKEND_ORIGIN,
  );

  return fetch(backendUrl, {
    method: request.method,
    headers: request.headers,
    body:
      request.method === "GET" || request.method === "HEAD"
        ? undefined
        : request.body,
  });
}

const server = serve({
  hostname: "0.0.0.0",
  port: 3000,
  routes: {
    "/api/v1/*": proxyApi,
    "/assets/*": proxyApi,
    // Serve index.html for all unmatched routes.
    "/*": index,
  },

  development: process.env.NODE_ENV !== "production" && {
    // Enable browser hot reloading in development
    hmr: true,

    // Echo console logs from the browser to the server
    console: true,
  },
});

console.log(`🚀 Server running at ${server.url}`);
