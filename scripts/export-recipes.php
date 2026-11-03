<?php
// Export static recipe configuration only; no database or player records are read.
$root = dirname(__DIR__);
$recipes = require $root . '/reference/upstream/Database/Laragon/ninjasage/app/Services/Amf/BlacksmithRecipes.php';
$directory = $root . '/database/catalogs';
if (!is_dir($directory)) mkdir($directory, 0777, true);
file_put_contents($directory . '/blacksmith.json', json_encode($recipes, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR) . "\n");
echo 'Exported ' . count($recipes) . " recipes from pinned upstream configuration.\n";
