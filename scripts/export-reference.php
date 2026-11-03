<?php
require __DIR__ . '/reference-env.php';
$connection = $app['db']->connection();
$schema = "-- Source: ecf9012d885d1c89fd5f42791eed4f37f90f2edd; MySQL 8.4.7 SHOW CREATE TABLE.\n-- Baseline migration workaround: session strict mode relaxed for event data inserts only.\n";
foreach ($connection->select('SHOW TABLES') as $row) {
    $table=array_values((array)$row)[0];
    $definition=(array)$connection->selectOne("SHOW CREATE TABLE `$table`");
    $ddl=array_values($definition)[1];
    $ddl=preg_replace('/ AUTO_INCREMENT=\d+/', '', $ddl);
    $ddl=str_replace('CREATE TABLE ', 'CREATE TABLE IF NOT EXISTS ', $ddl);
    $schema.=$ddl.";\n\n";
}
$directory=$root.'/database/schema';
if (!is_dir($directory)) mkdir($directory,0777,true);
file_put_contents($directory.'/mysql.sql',rtrim($schema)."\n");
$catalogs=[];
foreach (['items','skills','pets','missions','npcs','enemies','talents','senjutsus','game_configs','x_p_s'] as $table) {
    $catalogs[$table]=array_map(fn($row)=>(array)$row,$connection->select("SELECT * FROM `$table` ORDER BY id"));
}
file_put_contents($root.'/.tools/reference-catalogs.json',json_encode($catalogs,JSON_THROW_ON_ERROR));
echo 'Exported schema and '.array_sum(array_map('count',$catalogs))." catalog rows. No account records exported.\n";
