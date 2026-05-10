import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [tailwindcss()],
  publicDir: false,
  // server: {
  //   cors: {
  //     // the origin you will be accessing via browser
  //     origin: `${Deno.env.get("PROTO")}://${Deno.env.get("HOST")}`,
  //   },
  // },
  build: {
    manifest: true,
    emptyOutDir: false,
    outDir: "public",
    assetsDir: "./assets",
    rollupOptions: {
      // overwrite default .html entry
      input: "client/main.ts",
    },
  },
});
