<?php
namespace Illuminate\Database { class Seeder {} }
namespace App\Models { class GameEvent { public static array $rows=[]; public static function truncate(){} public static function create($row){self::$rows[]=$row;} } }
namespace { require __DIR__.'/../reference/upstream/Database/Laragon/ninjasage/database/seeders/GameEventSeeder.php'; (new \Database\Seeders\GameEventSeeder)->run();$gameData=json_decode(file_get_contents(__DIR__.'/../reference/upstream/Database/Laragon/ninjasage/public/game_data/gamedata.json'),true,flags:JSON_THROW_ON_ERROR);
$byId=array_column($gameData,'data','id');
foreach (\App\Models\GameEvent::$rows as &$row) {
    if (($row['panel']??'')==='FeastOfGratitudeMenu') $row['data']['bosses']=$byId['thanksgiving2025']['bosses'];
    if (($row['panel']??'')==='DragonHunt') $row['data']['bosses']=$byId['dragon_hunt']['bosses'];
}
unset($row);
file_put_contents(__DIR__.'/../database/catalogs/events.json',json_encode(\App\Models\GameEvent::$rows,JSON_PRETTY_PRINT|JSON_UNESCAPED_SLASHES|JSON_THROW_ON_ERROR));echo count(\App\Models\GameEvent::$rows)." events exported; no database accessed.\n"; }
