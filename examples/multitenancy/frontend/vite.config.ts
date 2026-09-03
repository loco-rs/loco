import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist" },
  server: { port: 5173, proxy: { "/api": "http://localhost:5150" } },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    environmentOptions: {
      jsdom: { url: "http://localhost" }
    },
    coverage: {
      provider: "v8",
      include: ["src/auth/session.ts", "src/api/client.ts"],
      thresholds: {
        branches: 100,
        functions: 100,
        lines: 100,
        statements: 100
      }
    }
  }
});
