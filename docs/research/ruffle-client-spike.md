# Ruffle client spike results (2026-11-03)

## Question

Can the Adobe AIR client (`NSCUSTOM.exe` wrapping `NinjaSage.swf`) be replaced by a
Rust-native Ruffle shell, so the whole project (server + client tooling) is Rust?

## Verdict

**Feasible.** The complete AIR-verified acceptance slice — login, character data,
attribute points, mission room, first mission battle, mission completion with
persisted rewards, and post-mission shopping — runs unmodified inside Ruffle
0.7.1 (desktop) against the Rust backend. See [progress](../migration/progress.md)
for what this does and does not gate.

## Client facts from static inspection

- `.tools/client/NinjaSage.swf`: 9.5 MB zipped, single AVM2/AS3 code blob
  (one `DoABC`), SWF version 51 (Harman AIR 51.1 repack), stage 1920x1080, 40 fps.
- All sub-SWFs (`items/`, `skills/`, `enemy/`, `npcs/`, `pets/`, `mission/`,
  `swfpanels/`, `consumables/`, `essentials/`, `materials/`) are pure-graphics
  assets: no bytecode, no AIR-specific tags.
- The whole engine ships in the main SWF with embedded AS3 libraries:
  `skein.engine.io.client` (Engine.IO 3 polling + WebSocket transports),
  `com.worlize.websocket`, `net.gimite`, `com.hurlant.crypto` (pure AS3),
  `br.com.stimuli.loading` (BulkLoader), `com.gestouch`.
- AIR-specific surface is tiny: one `flash.filesystem.FileStream` reference,
  `flash.display3D`/Stage3D absent, no `NativeWindow`, no `EncryptedLocalStore`.
- `--player-runtime air` matters: the client uses `NativeMenuItem`,
  `File.applicationDirectory`, `File.exists`, `File.applicationStorageDirectory`,
  all non-fatal stubs under that runtime.

## What runs

Verified on the Windows workstation with the pinned assets (`.tools/client`, the
`prepare-local-client.py` output) against the Rust game server on
`127.0.0.1:8080`:

1. Ruffle desktop 0.7.1 launch configuration:
   `ruffle.exe NinjaSage.swf --player-runtime air --tcp-connections allow
   --socket-allow 127.0.0.1:8080 --spoof-url http://127.0.0.1:8080/client/NinjaSage.swf
   --save-directory .tools/ruffle-save --cache-directory .tools/ruffle-cache
   --width 1280 --height 720`
   - `--base` is NOT wanted; keep the SWF on a `file://` URL in `.tools/client`
     so relative asset lookups hit the local client copy.
   - `--spoof-url` (any `http://` URL under the server origin) is required: with a
     `file://` movie, Ruffle refuses `SharedObject.getLocal` paths (`..` segments)
     and the `VersionCheckTask` init task crashed the boot chain.
2. Boot: version check, custom color API, event catalog, login screen, sounds,
   fonts and device-font fallback all initialize. Login screen renders and
   functions (registration validated client-side).
3. Gameplay: login (`SystemLogin.loginUser`), character list/data, attribute
   points (`setPoints`), mission room, battle start/finish with the base64
   battle report, mission rewards persisted, weapon purchase and equip.
4. `BulkLoader` fetches `http://127.0.0.1:8080/api/id-custom-color`.

## Server changes required (landed with the spike)

All in the existing workspace; headless tests still pass, plus live AIR-client
compatibility behaviour was preserved rather than replaced:

- `crates/amf-codec`: accept AMF0 marker `0x0D` (Unsupported) as `Value::Null`.
  Flash Player serialises values with no AMF0 representation (e.g. Dictionary)
  with that marker; Ruffle sends it and the original SabreAMF server tolerated it.
- `apps/game-server/src/account.rs` `from_amf`: `Value::MixedArray` with purely
  sequential numeric keys now decodes as a positional list. PHP arrays behave the
  same way, and Ruffle wraps login/mission arguments in ECMA arrays (dense AIR
  arrays do this too).
- `apps/game-server/src/account.rs` `string()`: whole floats no longer stringify
  as `299434.0`; PHP printed them without the decimal tail and Ruffle sends some
  AIR string fields as numbers (the login IV relied on this).
- `apps/game-server/src/events.rs` (new) plus dispatch arm: `Events.get`, the
  event catalog the client requests at boot, mirroring
  `app/Services/Amf/EventsService/CatalogService.php` including optional
  character-session validation.

## Remaining known noise (benign today)

- `flash.filesystem.File` / `NativeMenuItem` stub warnings at boot (AIR-only).
- `ReferenceError #1065 FileMode is not defined` from AIR-only code paths in
  complete-event handlers; gameplay continues despite them.
- `flash.utils.Dictionary` weak-key constructor falls back to strong keys.
- `Capabilities.serverString` getter stub.
- click sound uses an `undefined`-prefixed relative URL (client builds from a
  value whose Ruffle-side provider differs); only the click sound is missing.
- `Analyzer.libraries`-style `Analytics.libraries` call returns the standard
  "method is not implemented" envelope — the original server has no Analytics
  service either, so behaviour matches. New methods surface loudly via the
  existing `AMF method not implemented` warning.

## Production shape (next phase, not part of the spike)

Two viable integration paths, in preference order:

1. **Thin Rust shell on `ruffle_frontend_utils`**: embed the player with a
   pinned configuration (AIR runtime, loopback socket allow-list, spoofed
   origin, no CLI/permits UI), own winit window, and reuse `shared` process
   scaffolding. Fits the workspace's modular-monolith layout.
2. **Vendor the stock desktop player** with a pre-seeded config — fastest but
   bundles the full GUI/explorer shell.

Either way the AIR runtime dependency disappears; the client becomes another
Cargo artifact.

## Follow-ups

- Extend the same launch recipe into a `scripts/start-ruffle-client.ps1`
  convenience runner using only ignored local paths.
- Explore Ruffle's `ExternalInterface` provider as a backend-bridge for
  diagnostics and capabilities (would likely also restore the click-sound URL
  and remove the FileMode errors).
- Ruffle nightly builds move daily; re-verify after upgrades.
- Keep real-client acceptance checks in `tools/client` (AIR) until Ruffle
  covers the same breadth; the headless AMF/MySQL suite remains the default
  regression workflow.
