<?php
// Run from any directory. Uses the pinned upstream codec without Laravel/Composer.
error_reporting(E_ALL & ~E_DEPRECATED);
$source = dirname(__DIR__, 3) . '/reference/upstream/Database/Laragon/ninjasage/app/ThirdParty/';
spl_autoload_register(function ($class) use ($source) {
    $path = $source . str_replace('_', '/', $class) . '.php';
    if (is_file($path)) require_once $path;
});
function writeFixture($name, $wire) {
    $path=__DIR__ . '/fixtures/' . $name . '.amf';
    if(is_file($path) && file_get_contents($path)===$wire)return;
    if(file_put_contents($path,$wire)===false)throw new Exception('Cannot write fixture: '.$name);
}
function saveFixture($name, $version, $bodies) {
    $message = new SabreAMF_Message();
    $message->setEncoding($version);
    foreach ($bodies as $body) $message->addBody($body);
    $stream = new SabreAMF_OutputStream();
    $message->serialize($stream);
    writeFixture($name,$stream->getRawData());
}
saveFixture('amf0-multiple', 0, [
    ['target'=>'SystemLogin.checkVersion', 'response'=>'/1', 'data'=>['1.0.0', true, null, 42.5]],
    ['target'=>'Unknown.method', 'response'=>'/2', 'data'=>['key'=>'value']],
]);
saveFixture('amf3-request', 3, [
    ['target'=>'SystemLogin.checkVersion', 'response'=>'/1', 'data'=>['1.0.0', -268435456, 268435455, true, null]],
]);
saveFixture('amf3-bytes',3,[['target'=>'Analytics.libraries','response'=>'/1','data'=>[new SabreAMF_ByteArray("\x00\x01\xfftest")]]]);
saveFixture('amf3-response', 0, [
    ['target'=>'/1/onResult', 'response'=>'', 'data'=>new SabreAMF_AMF3_Wrapper((object)['status'=>500, 'error'=>'Unknown.method failed'])],
]);
// Sabre's serializer never emits object/trait references. Construct reference-bearing
// wire packets explicitly and verify their semantics with its pinned deserializer.
function verifyReferenceFixture($name, $version, $payload, $expected) {
    $wire = pack('nnn', $version, 0, 1)
        . pack('n', 4) . 'test' . pack('n', 2) . '/1'
        . hex2bin('ffffffff') . $payload;
    $message = new SabreAMF_Message();
    $message->deserialize(new SabreAMF_InputStream($wire));
    $actual = $message->getBodies()[0]['data'];
    if (json_encode($actual) !== json_encode($expected)) {
        throw new Exception('Reference fixture mismatch: ' . $name);
    }
    writeFixture($name,$wire);
}
verifyReferenceFixture('amf3-references', 3,
    hex2bin('11090b010a1301037804070a0104090a02090301040b0906'),
    [(object)['x'=>7], (object)['x'=>9], (object)['x'=>7], [11], [11]]);
verifyReferenceFixture('amf0-references', 0,
    hex2bin('0a000000040300017800401c0000000000000000090700010a000000010101070002'),
    [['x'=>7], ['x'=>7], [true], [true]]);
echo "Generated three serializer fixtures and two deserializer-verified reference fixtures.\n";
