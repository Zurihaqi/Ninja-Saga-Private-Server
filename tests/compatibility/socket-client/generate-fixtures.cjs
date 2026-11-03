const fs = require('node:fs');
const path = require('node:path');
const parser = require('engine.io-parser');
const packets = [
  { type: 'ping', data: 'probe' },
  { type: 'message', data: '0/compat,' },
  { type: 'message', data: '2/compat,["echo","忍者😀",{"value":1}]' },
  { type: 'close' }
];
parser.encodePayload(packets, false, encoded => {
  const output = {
    provenance: 'Synthetic packets encoded by engine.io-parser ' + require('engine.io-parser/package.json').version,
    protocol: parser.protocol,
    packets,
    encoded
  };
  fs.writeFileSync(path.join(__dirname, '../../../crates/socket-protocol/tests/fixtures/engine-v3.json'), JSON.stringify(output, null, 2) + '\n');
});
