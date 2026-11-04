# Static gameplay configuration

`blacksmith.json` contains 65 recipes exported without modification from `app/Services/Amf/BlacksmithRecipes.php` at pinned upstream commit `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`.

Regenerate with `php scripts/export-recipes.php` after materializing the reference checkout. It is embedded in the Rust binary, so runtime PHP and the reference source file are unnecessary. Other game catalogs are imported explicitly into MySQL or served from `GAME_DATA_ROOT`; player records are never included here.
