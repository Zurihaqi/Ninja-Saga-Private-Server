<?php
// Creates only a synthetic development account against the loopback Rust gateway.
$root=dirname(__DIR__);
$source=$root.'/reference/upstream/Database/Laragon/ninjasage/app/ThirdParty/';
spl_autoload_register(function($class)use($source){$path=$source.str_replace('_','/',$class).'.php';if(is_file($path))require_once $path;});
error_reporting(E_ALL & ~E_DEPRECATED);
$file=$root.'/.tools/local-account.json';
$account=is_file($file)?json_decode(file_get_contents($file),true):['username'=>'local_ninja','password'=>'NinjaTest123!'];
$message=new SabreAMF_Message();$message->setEncoding(3);
$message->addBody(['target'=>'SystemLogin.registerUser','response'=>'/1','data'=>[$account['username'],'local_ninja@example.invalid',$account['password'],'local-development']]);
$stream=new SabreAMF_OutputStream();$message->serialize($stream);
$response=file_get_contents('http://127.0.0.1:8080/amf',false,stream_context_create(['http'=>['method'=>'POST','header'=>'Content-Type: application/x-amf','content'=>$stream->getRawData()]]));
$result=new SabreAMF_Message();$result->deserialize(new SabreAMF_InputStream($response));
$data=(array)$result->getBodies()[0]['data'];
if($data['status']!==1 && ($data['result']??'')!=='Username already exists!')throw new RuntimeException('Local registration failed');
file_put_contents($file,json_encode($account,JSON_PRETTY_PRINT));
echo "Development account ready. Login details: .tools/local-account.json\n";
