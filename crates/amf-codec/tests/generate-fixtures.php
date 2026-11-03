<?php
// Run from any directory. Uses the pinned upstream codec without Laravel/Composer.
error_reporting(E_ALL & ~E_DEPRECATED);
$source = dirname(__DIR__, 3) . '/reference/upstream/Database/Laragon/ninjasage/app/ThirdParty/';
spl_autoload_register(function ($class) use ($source) {
    $path = $source . str_replace('_', '/', $class) . '.php';
    if (is_file($path)) require_once $path;
});
function saveFixture($name, $version, $bodies) {
    $message = new SabreAMF_Message();
    $message->setEncoding($version);
    foreach ($bodies as $body) $message->addBody($body);
    $stream = new SabreAMF_OutputStream();
    $message->serialize($stream);
    file_put_contents(__DIR__ . '/fixtures/' . $name . '.amf', $stream->getRawData());
}
saveFixture('amf0-multiple', 0, [
    ['target'=>'SystemLogin.checkVersion', 'response'=>'/1', 'data'=>['1.0.0', true, null, 42.5]],
    ['target'=>'Unknown.method', 'response'=>'/2', 'data'=>['key'=>'value']],
]);
saveFixture('amf3-request', 3, [
    ['target'=>'SystemLogin.checkVersion', 'response'=>'/1', 'data'=>['1.0.0', -268435456, 268435455, true, null]],
]);
saveFixture('amf3-response', 0, [
    ['target'=>'/1/onResult', 'response'=>'', 'data'=>new SabreAMF_AMF3_Wrapper((object)['status'=>500, 'error'=>'Unknown.method failed'])],
]);
echo "Generated three SabreAMF fixtures.\n";
