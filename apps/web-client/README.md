# Browser game and admin dashboard

This React host loads the original SWF in self-hosted Ruffle 0.7.1. It also provides the administrative UI. Game logic remains in the original SWF and Rust backend; this is not a rewrite of the game into React.

```powershell
npm ci --prefix apps/web-client --ignore-scripts --no-audit --no-fund
npm run build --prefix apps/web-client
```

The prebuild step copies Ruffle JavaScript, WebAssembly and licenses from the locked npm dependency into `public/ruffle`; Vite then copies them into `dist`. Both generated directories are ignored by Git. PHP and Node services are unnecessary at runtime; Node is a build tool here.

Set `WEB_CLIENT_ROOT` to the absolute `apps/web-client/dist` directory and start the game backend. On the configured workstation, `scripts/start-local.ps1` sets this automatically.

- Game: `http://127.0.0.1:8080/app/`
- Admin: `http://127.0.0.1:8080/admin` → `/app/#admin`
- Vite development host: `npm run dev --prefix apps/web-client`, then `http://localhost:5173/app/`. The configured proxy forwards game requests to port 8080.

The backend also needs its database, original catalogs and prepared client/assets. Use `CLIENT_ASSET_ROOT` for the patched client copy and `GAME_DATA_ROOT` for the original catalogs. A frontend build alone does not provide the original assets.

Admin login accepts a bcrypt-backed account with `account_type=99` or an explicitly configured `ADMIN_SECRET`. The local helper creates a unique secret at ignored `.tools/admin-secret.txt` if none is configured. Read that file locally and enter its value in the dashboard's secret field. There is no built-in admin password; do not commit or share the secret.

Actual-client coverage beyond the first-mission slice remains in progress. The browser app building successfully does not verify all SWF panels or multiplayer behavior.
