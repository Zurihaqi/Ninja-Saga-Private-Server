import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The Rust game server on 127.0.0.1:8080 serves the same built app at /app in
// production; in development Vite keeps the same /app base and proxies every
// game prefix across so the SWF sees one same-origin http source.
const backend = "http://127.0.0.1:8080";

const proxiedPaths = [
    "/amf",
    "/api",
    "/client",
    "/crossdomain.xml",
    "/gateway.php",
    "/game_data",
    "/NinjaSage.swf",
    "/sounds",
    "/swfpanels",
    "/skills",
    "/items",
    "/enemy",
    "/mission",
    "/npcs",
    "/pets",
    "/consumables",
    "/essentials",
    "/materials",
    "/Adobe AIR",
];

const proxy = Object.fromEntries(
    proxiedPaths.map((path) => [path, { target: backend, changeOrigin: true }]),
);

export default defineConfig({
    base: "/app/",
    plugins: [react()],
    server: {
        port: 5173,
        proxy,
    },
});
