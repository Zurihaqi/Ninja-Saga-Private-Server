<?php
/** Generate HTTP body fixtures with the pinned upstream codec and version service. */
declare(strict_types=1);

// The legacy codec creates dynamic properties and passes null to strlen().
// Exclude deprecations only; warnings and errors remain visible.
error_reporting(E_ALL & ~E_DEPRECATED);
$root = dirname(__DIR__, 2);
$source = $root . '/reference/upstream/Database/Laragon/ninjasage/';
spl_autoload_register(function (string $class) use ($source): void {
    if (str_starts_with($class, 'SabreAMF_')) {
        $path = $source . 'app/ThirdParty/' . str_replace('_', '/', $class) . '.php';
        if (is_file($path)) {
            require_once $path;
        }
    }
});
require_once $source . 'app/Services/Amf/GameService/VersionService.php';
$service = new App\Services\Amf\GameService\VersionService();
$output = __DIR__ . '/fixtures';
if (!is_dir($output) && !mkdir($output, 0777, true) && !is_dir($output)) {
    throw new RuntimeException('Could not create fixtures directory');
}

/** Matches AmfController::normalizeAmf3Data at the pinned source commit. */
function normalizeAmf3Data(mixed $data): mixed
{
    if (!is_array($data)) {
        return $data;
    }
    if (array_is_list($data)) {
        return array_map('normalizeAmf3Data', $data);
    }
    $object = new stdClass();
    foreach ($data as $key => $value) {
        $object->{$key} = normalizeAmf3Data($value);
    }
    return $object;
}

function serializeMessage(SabreAMF_Message $message): string
{
    $stream = new SabreAMF_OutputStream();
    $message->serialize($stream);
    return $stream->getRawData();
}

$manifest = [
    'reference_commit' => 'ecf9012d885d1c89fd5f42791eed4f37f90f2edd',
    'kind' => 'generated source-derived fixtures; not live HTTP/client captures',
    'sources_sha256' => [],
    'files' => [],
];
foreach ([
    'app/Services/Amf/GameService/VersionService.php',
    'app/Http/Controllers/AmfController.php',
    'app/ThirdParty/SabreAMF/Message.php',
    'app/ThirdParty/SabreAMF/AMF0/Serializer.php',
    'app/ThirdParty/SabreAMF/AMF3/Serializer.php',
] as $path) {
    $manifest['sources_sha256'][$path] = hash_file('sha256', $source . $path);
}

foreach ([0, 3] as $encoding) {
    $request = new SabreAMF_Message();
    $request->setEncoding($encoding);
    $request->addBody(['target' => 'Game.getVersion', 'response' => '/1', 'data' => []]);
    $requestBytes = serializeMessage($request);

    // Derive encoding and response URI from an actual upstream decode, just as
    // the controller does, rather than assuming the request header is enough.
    $decoded = new SabreAMF_Message();
    $decoded->deserialize(new SabreAMF_InputStream($requestBytes));
    $useWrapper = $decoded->getEncoding() === SabreAMF_Const::AMF3;
    $response = new SabreAMF_Message();
    $response->setEncoding($useWrapper ? SabreAMF_Const::AMF0 : $decoded->getEncoding());
    foreach ($decoded->getBodies() as $body) {
        if ($body['target'] !== 'Game.getVersion' || $body['data'] !== []) {
            throw new RuntimeException('Unexpected fixture request after decode');
        }
        $result = $service->getVersion();
        $response->addBody([
            'target' => $body['response'] . '/onResult',
            // Controller uses null; SabreAMF serializes it as an empty URI.
            'response' => null,
            'data' => $useWrapper
                ? new SabreAMF_AMF3_Wrapper(normalizeAmf3Data($result))
                : $result,
        ]);
    }
    foreach (['request' => $requestBytes, 'response' => serializeMessage($response)] as $kind => $bytes) {
        $name = "game-version-amf{$encoding}-{$kind}.amf";
        if (file_put_contents($output . '/' . $name, $bytes) !== strlen($bytes)) {
            throw new RuntimeException('Failed writing ' . $name);
        }
        $manifest['files'][$name] = ['bytes' => strlen($bytes), 'sha256' => hash('sha256', $bytes)];
    }
}
file_put_contents($output . '/manifest.json', json_encode($manifest, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR) . "\n");
echo "Generated four Game.getVersion HTTP-body fixtures and provenance manifest.\n";
