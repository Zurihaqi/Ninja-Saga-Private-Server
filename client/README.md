# Prepare the compatible client

These scripts are required client build inputs. Downloaded AIR movies, Java/FFDec binaries, generated artifacts and credentials are excluded from Git. The servers themselves require neither Java nor PHP.

1. Materialize the original client at `reference/upstream/Custom Client/NS Custom Client V1` using the preserved `legacy/php-node` branch. Do not modify that reference copy.

2. Install Java and place the FFDec compiler at `.tools/ffdec/ffdec.jar` (the verified local tool was version 26.4.0).

3. Run `python client/prepare-local-client.py` from this repository. It copies the client into ignored `.tools/client`, redirects local service/merchant endpoints and compiles all panel repairs.

4. Set `CLIENT_ASSET_ROOT` to that repaired directory and supply the original public game data through `GAME_DATA_ROOT`.

5. Launch `.tools/client/NSCUSTOM.exe` or reload the browser host. See [runtime setup](../docs/runtime-setup.md).

For an existing local copy, run `python client/patch-client-panels.py`. The helper compiles a temporary candidate, re-exports bytecode to check all repair markers, backs up the original local movie once, and replaces only verified output. `--check` verifies an installed copy without editing it.

The repairs cover bounded event-banner startup, skill inventory/loading and presets, Pet Villa/timer/model cleanup, and free former paid products. No real-money checkout remains in these product actions. Missing upstream art is not fabricated; seven skill movies remain a content gap.

Villa static previews use generated silent movies under `.tools/client/pet-previews` so pet sound effects cannot overlap or leak into other panels. Combat loads the original pet movies. Preparation generates the previews automatically; `--check` verifies their hashes and compiled routing. Close and reopen the game after updating to clear sounds started by the previous client.
