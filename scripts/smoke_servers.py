"""Exercise built service binaries over loopback; no database or external services."""
import os
from pathlib import Path
import socket
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def request(url, data=None):
    try:
        with urllib.request.urlopen(url, data=data, timeout=2) as response:
            return response.status, response.read(), response.headers
    except urllib.error.HTTPError as response:
        return response.code, response.read(), response.headers


for service in ("game-server", "chat-server", "pvp-server"):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        port = sock.getsockname()[1]
    executable = ROOT / "target" / "debug" / (service + (".exe" if os.name == "nt" else ""))
    env = dict(os.environ, BIND_ADDR=f"127.0.0.1:{port}")
    process = subprocess.Popen([str(executable)], env=env, stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL,
                               creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
    base = f"http://127.0.0.1:{port}"
    try:
        for attempt in range(100):
            if process.poll() is not None:
                raise RuntimeError(f"{service} exited with {process.returncode}")
            try:
                status, body, _ = request(base + "/health/live")
                break
            except urllib.error.URLError:
                time.sleep(0.05)
        else:
            raise RuntimeError(f"{service} startup timeout")
        assert (status, body) == (200, b"ok")
        assert request(base + "/health/ready")[0] == 503
        if service == "game-server":
            assert request(base + "/amf")[:2] == (200, b"AMF Gateway Ready")
            for version in (0, 3):
                fixtures = ROOT / "tests" / "compatibility" / "fixtures"
                payload = (fixtures / f"game-version-amf{version}-request.amf").read_bytes()
                expected = (fixtures / f"game-version-amf{version}-response.amf").read_bytes()
                for route in ("/amf", "/gateway.php"):
                    status, body, headers = request(base + route, payload)
                    assert status == 200 and body == expected
                    assert headers["Content-Type"] == "application/x-amf"
        print(f"PASS {service}: live HTTP checks")
    finally:
        process.terminate()
        process.wait(timeout=5)
