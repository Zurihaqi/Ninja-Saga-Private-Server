# Pinned behavioral reference

Source: https://github.com/zurihaqi/Ninja-Saga-Private-Server

Commit: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd` (verified against remote HEAD on 2026-11-03).

`upstream/` is an ignored, detached, sparse Git checkout. Backend source, public game-data catalogs, the original custom client/AIR runtime and its assets are materialized. Locked Composer dependencies are installed for baseline migration/fixture work; generated vendor/cache files are local only. No tracked upstream source was modified. The playable runtime uses Rust; PHP/Node services are not required for the verified first-mission flow. Never reuse committed upstream credentials.

Recreate from the workspace root:

```powershell
git clone --filter=blob:none --no-checkout https://github.com/zurihaqi/Ninja-Saga-Private-Server.git reference/upstream
git -C reference/upstream sparse-checkout set Database/Laragon/ninjasage/app Database/Laragon/ninjasage/bootstrap Database/Laragon/ninjasage/config Database/Laragon/ninjasage/database Database/Laragon/ninjasage/routes Database/Laragon/ninjasage/tests Database/Laragon/ninjasage/chat-server Database/Laragon/ninjasage/pvp-server
git -C reference/upstream checkout --detach ecf9012d885d1c89fd5f42791eed4f37f90f2edd
git -C reference/upstream sparse-checkout add Database/Laragon/ninjasage/public/game_data "Custom Client/NS Custom Client V1" "QoL Tools"
```

Upstream licensing and asset distribution rights need review before redistribution; the checkout is deliberately excluded from the Rust repository.
