# Reference static validation

Pinned source: `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Regenerate from the workspace root with `python scripts/check_reference.py`.

Only parser checks were run: `php -l` for first-party app/routes/database/tests PHP (excluding app/ThirdParty), and `node --check` for chat/PvP JavaScript (excluding node_modules). Neither command executes application code. No database connection, seeder, dependency install, HTTP request or game client was used.

Tools: `PHP 8.5.11 (cli) (built: Sep 22 2026 13:50:46) (ZTS Visual C++ 2022 x64)`; `v26.3.0`.

Ordered path/content SHA-256 for checked inputs: `1c64b18fae7b5481019d03c6f84e9d928170d4e1a8db5a6fbe4e887246b61ef2`.

| Language | Files | Passed | Syntax failures | Check errors |
|---|---:|---:|---:|---:|
| PHP | 388 | 388 | 0 | 0 |
| JavaScript | 18 | 18 | 0 | 0 |

## Failures

None.

## Limits

Passing syntax checks does not establish dependency compatibility, Laravel bootability, migration correctness, authorization, game behavior or client/protocol compatibility. PHP runtime version may differ from the upstream minimum. Full baseline execution remains a separate acceptance gate.
