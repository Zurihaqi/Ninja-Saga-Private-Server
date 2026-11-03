# Static gameplay configuration

`blacksmith.json` contains 65 recipes exported without modification from `app/Services/Amf/BlacksmithRecipes.php` at pinned upstream commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`.

Regeneration uses `php scripts/export-recipes.php` on the `sprint/1-protocol-compatibility` development branch after materializing the reference checkout. That tooling is excluded from this runtime branch. Recipes are embedded in the Rust binary, so runtime PHP and the reference source file are unnecessary. Other catalogs are imported explicitly into MySQL or served from `GAME_DATA_ROOT`; player records are never included here.
