# Pinned behavioral reference

Source: https://github.com/zurihaqi/Ninja-Saga-Private-Server

Commit: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd` (verified against remote HEAD on 2026-11-03).

`upstream/` is an ignored, detached, sparse Git checkout. Backend app, config, bootstrap, migrations, tests, routes, chat and PvP source are materialized. Game assets, AIR runtime, public assets and storage data are not downloaded. Never use committed keys or environment credentials for a deployment. This checkout is unmodified and is not a running baseline.

Recreate from the workspace root:

```powershell
git clone --filter=blob:none --no-checkout https://github.com/zurihaqi/Ninja-Saga-Private-Server.git reference/upstream
git -C reference/upstream sparse-checkout set Database/Laragon/ninjasage/app Database/Laragon/ninjasage/bootstrap Database/Laragon/ninjasage/config Database/Laragon/ninjasage/database Database/Laragon/ninjasage/routes Database/Laragon/ninjasage/tests Database/Laragon/ninjasage/chat-server Database/Laragon/ninjasage/pvp-server
git -C reference/upstream checkout --detach ecf9012d885d1c89fd5f42791eed4f37f90f2edd
```

Upstream licensing and asset distribution rights need review before redistribution; the checkout is deliberately excluded from the Rust repository.
