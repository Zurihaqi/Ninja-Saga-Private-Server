// Development-only interoperability check; production services do not require Node.
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const net = require('node:net');
const path = require('node:path');
const io = require('socket.io-client');

function once(emitter, event, timeout = 5000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { emitter.removeListener(event, done); reject(new Error(`timeout: ${event}`)); }, timeout);
    function done(...args) { clearTimeout(timer); resolve(args); }
    emitter.once(event, done);
  });
}

async function freePort() {
  const server = net.createServer();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  await new Promise(resolve => server.close(resolve));
  return port;
}

async function main() {
  const root = path.resolve(__dirname, '../../..');
  const port = await freePort();
  const base = `http://127.0.0.1:${port}`;
  const child = spawn(path.join(root, 'target/debug/protocol-spike' + (process.platform === 'win32' ? '.exe' : '')), [], {
    env: { ...process.env, BIND_ADDR: `127.0.0.1:${port}` }, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe']
  });
  let failure;
  let logs = '';
  child.on('error', error => { failure = error; });
  child.stdout.on('data', data => { logs = (logs + data).slice(-4096); });
  child.stderr.on('data', data => { logs = (logs + data).slice(-4096); });
  const sockets = [];
  try {
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      if (failure) throw failure;
      if (child.exitCode !== null) throw new Error(`spike exited: ${logs}`);
      try { ready = (await fetch(base + '/health/live')).ok; } catch (_) {}
      if (ready) break;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert(ready, `startup timeout: ${logs}`);
    for (let index = 0; index < 2; index++) {
      const socket = io(base + '/compat', { transports: ['polling'], upgrade: false, forceNew: true, reconnection: false, autoConnect: false });
      sockets.push(socket);
      const connected = once(socket, 'connect');
      socket.open();
      await connected;
    }
    assert.notEqual(sockets[0].io.engine.id, sockets[1].io.engine.id);
    const received = [[], []];
    sockets.forEach((socket, index) => socket.on('echo', (...args) => received[index].push(args)));
    const echoes = sockets.map(socket => once(socket, 'echo'));
    sockets[0].emit('echo', '忍者😀', { value: 1 });
    sockets[1].emit('echo', 'second client', [1, 2]);
    assert.deepEqual(await echoes[0], ['忍者😀', { value: 1 }]);
    assert.deepEqual(await echoes[1], ['second client', [1, 2]]);
    assert.equal(received[0].length, 1);
    assert.equal(received[1].length, 1);

    const engine = sockets[0].io.engine;
    const pong = once(engine, 'pong');
    engine.ping();
    await pong;
    const previousId = engine.id;
    const closed = once(engine, 'close');
    sockets[0].close();
    // The legacy manager closes asynchronously after flushing disconnect frames.
    // Reopening before that close event races its old-engine cleanup.
    await closed;
    const connected = once(sockets[0], 'connect');
    sockets[0].open();
    await connected;
    assert.notEqual(sockets[0].io.engine.id, previousId);
    console.log('PASS Socket.IO 2.5.0 / Engine.IO 3: namespace, Unicode echo, isolation, ping/pong, reconnect');
  } finally {
    sockets.forEach(socket => socket.close());
    const exited = once(child, 'exit', 5000);
    child.kill();
    await exited;
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
