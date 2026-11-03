<?php
// Isolated local baseline only. Never points at an existing deployment.
$root = dirname(__DIR__);
$stateFile = $root . '/.tools/database.json';
if (!is_file($stateFile)) {
    $db = new PDO('mysql:host=127.0.0.1;port=3307', 'root', '');
    $password = bin2hex(random_bytes(24));
    $rootPassword = bin2hex(random_bytes(32));
    foreach (['ninja_reference', 'ninja_rust_test'] as $name) {
        $db->exec("CREATE DATABASE `$name` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci");
    }
    $db->exec("CREATE USER 'ninja_local'@'localhost' IDENTIFIED BY '$password'");
    $db->exec("GRANT ALL ON ninja_reference.* TO 'ninja_local'@'localhost'");
    $db->exec("GRANT ALL ON ninja_rust_test.* TO 'ninja_local'@'localhost'");
    $db->exec("ALTER USER 'root'@'localhost' IDENTIFIED BY '$rootPassword'");
    file_put_contents($stateFile, json_encode(['username'=>'ninja_local', 'password'=>$password, 'root_password'=>$rootPassword], JSON_PRETTY_PRINT));
}
$state = json_decode(file_get_contents($stateFile), true, flags: JSON_THROW_ON_ERROR);
$source = $root . '/reference/upstream/Database/Laragon/ninjasage';
require $source . '/vendor/autoload.php';
$app = require $source . '/bootstrap/app.php';
$kernel = $app->make(Illuminate\Contracts\Console\Kernel::class);
// Set all baseline values explicitly; do not load upstream credentials.
foreach (['APP_ENV'=>'local','APP_DEBUG'=>'false','APP_KEY'=>'base64:'.base64_encode(random_bytes(32)),
    'DB_CONNECTION'=>'mysql','DB_HOST'=>'127.0.0.1','DB_PORT'=>'3307','DB_DATABASE'=>'ninja_reference',
    'DB_USERNAME'=>$state['username'],'DB_PASSWORD'=>$state['password'], 'LOG_CHANNEL'=>'stderr',
    'CACHE_STORE'=>'array','SESSION_DRIVER'=>'array', 'APP_URL'=>'http://127.0.0.1:8080'] as $key=>$value) {
    putenv("$key=$value"); $_ENV[$key] = $value; $_SERVER[$key] = $value;
}
foreach (['storage/framework/cache/data','storage/framework/sessions','storage/framework/views','storage/logs','bootstrap/cache'] as $dir) {
    if (!is_dir("$source/$dir")) mkdir("$source/$dir", 0777, true);
}
$kernel->bootstrap();
// Upstream's Dragon Gacha migration omits this required column. Preserve the
// declared schema; supply a temporary session-local default via SQL mode only
// for baseline migrations. Record the deviation rather than modifying sources.
$app['db']->connection()->statement("SET SESSION sql_mode = 'NO_ENGINE_SUBSTITUTION'");
echo "Baseline migration workaround: session strict mode disabled for upstream's missing game_events.image_url.\n";
if ($kernel->call('migrate', ['--force'=>true]) !== 0) {
    echo $kernel->output(); throw new RuntimeException('Reference migration failed');
}
echo $kernel->output();
// Seeders expect a private copy of the versioned public catalogs. Refuse missing input.
foreach (['library','skills','pet','mission','npc','enemy','talents','senjutsu','gamedata'] as $name) {
    if (!is_file("$source/public/game_data/$name.json")) throw new RuntimeException("Missing catalog: $name");
}
$app['config']->set('filesystems.disks.local.root', "$source/public");
foreach (['GameDataSeeder', 'XPSeeder', 'TalentSeeder'] as $name) {
    if ($kernel->call('db:seed', ['--class'=>"Database\\Seeders\\$name", '--force'=>true]) !== 0) {
        echo $kernel->output(); throw new RuntimeException("Reference seed failed: $name");
    }
    echo $kernel->output();
}
echo "Isolated reference schema and catalogs ready. Credentials remain in ignored .tools/database.json.\n";
