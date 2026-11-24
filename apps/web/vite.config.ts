import { readFileSync } from "node:fs";

import tailwindcss from "@tailwindcss/vite";
import basicSsl from "@vitejs/plugin-basic-ssl";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig(({ command }) => {
  const cert = process.env.DEV_TLS_CERT;
  const key = process.env.DEV_TLS_KEY;
  if (command === "serve" && Boolean(cert) !== Boolean(key)) {
    throw new Error("Setel DEV_TLS_CERT dan DEV_TLS_KEY bersama-sama.");
  }

  return {
    plugins: [react(), tailwindcss(), ...(command === "serve" && !cert ? [basicSsl()] : [])],
    server: {
      host: "127.0.0.1",
      https: command === "serve" && cert && key
        ? { cert: readFileSync(cert), key: readFileSync(key) }
        : undefined,
      proxy: {
        "/api": {
          target: process.env.DEV_API_ORIGIN ?? "http://127.0.0.1:3000",
          changeOrigin: true,
          secure: false,
        },
      },
    },
  };
});
