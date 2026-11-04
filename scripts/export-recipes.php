<?php
// Export static recipe configuration only; no database or player records are read.
$root = dirname(__DIR__);
$amf = $root . '/reference/upstream/Database/Laragon/ninjasage/app/Services/Amf';
$recipes = require $amf . '/BlacksmithRecipes.php';
$hunting = require $amf . '/HuntingHouseService/HuntingForgeRecipes.php';
$materialMarket = require $amf . '/MaterialMarketRecipes.php';
$directory = $root . '/database/catalogs';
if (!is_dir($directory)) mkdir($directory, 0777, true);
$flags = JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR;
file_put_contents($directory . '/blacksmith.json', json_encode($recipes, $flags) . "\n");
$exportOrdered = static function (array $map): array {
    $out = [];
    foreach ($map as $item => $entry) {
        $row = ['item' => $item, 'materials' => $entry['materials'], 'qty' => $entry['qty']];
        if (array_key_exists('end', $entry)) {
            $row['end'] = $entry['end'];
        }
        $out[] = $row;
    }
    return $out;
};
file_put_contents($directory . '/hunting_forge.json', json_encode($exportOrdered($hunting), $flags) . "\n");
file_put_contents($directory . '/material_market.json', json_encode($exportOrdered($materialMarket), $flags) . "\n");
echo 'Exported ' . count($recipes) . ' blacksmith, ' . count($hunting) . ' hunting forge, and ' . count($materialMarket) . " material market recipes.\n";
