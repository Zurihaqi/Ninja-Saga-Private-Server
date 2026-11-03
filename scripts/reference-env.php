<?php
// Shared local-only reference bootstrap; credentials never leave ignored .tools.
$root = dirname(__DIR__);
$source = $root . '/reference/upstream/Database/Laragon/ninjasage';
foreach (['storage/framework/cache/data','storage/framework/sessions','storage/framework/views','storage/logs','bootstrap/cache'] as $dir) {
    if (!is_dir("$source/$dir")) mkdir("$source/$dir",0777,true);
}
$state = json_decode(file_get_contents($root . '/.tools/database.json'), true, flags: JSON_THROW_ON_ERROR);
foreach (['APP_ENV'=>'testing','APP_DEBUG'=>'false','APP_KEY'=>'base64:'.base64_encode(str_repeat('t',32)),
    'DB_CONNECTION'=>'mysql','DB_HOST'=>'127.0.0.1','DB_PORT'=>'3307','DB_DATABASE'=>'ninja_reference',
    'DB_USERNAME'=>$state['username'],'DB_PASSWORD'=>$state['password'], 'LOG_CHANNEL'=>'null',
    'CACHE_STORE'=>'array','SESSION_DRIVER'=>'array','APP_URL'=>'http://127.0.0.1:8080'] as $key=>$value) {
    putenv("$key=$value"); $_ENV[$key]=$value; $_SERVER[$key]=$value;
}
require $source . '/vendor/autoload.php';
$app = require $source . '/bootstrap/app.php';
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
return $app;
