import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 6100, strictPort: true },
  build: {
    rollupOptions: {
      // Due pagine: impostazioni (React) e overlay (TS puro, sempre in memoria).
      input: { main: "index.html", overlay: "overlay.html" },
    },
  },
});
