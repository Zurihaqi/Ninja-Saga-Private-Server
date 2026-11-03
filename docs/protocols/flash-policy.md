# Flash policy source contract

Reference: `chat-server/index.js` at `ecf9012d885d1c89fd5f42791eed4f37f90f2edd`. Source inspection only.

Chat opens a raw TCP listener on 843 independently of database initialization. Each socket has a 3000ms timeout. It examines only the first `data` event; if its UTF-8 conversion contains `<policy-file-request/>`, it writes the policy plus a NUL byte, then destroys the socket. A split request across TCP chunks can fail, and immediate destruction after write needs runtime verification. Bind failure logs a warning and leaves chat startup running.

The HTTP route `/crossdomain.xml` returns the same XML without a NUL, HTTP 200, content type `text/x-cross-domain-policy`. Both access and HTTP-header elements allow wildcard domains; socket ports and HTTP headers are wildcards and `secure="false"`.

PvP has no equivalent policy handler in its entry point. The chat listener may supply host-wide policy in a shared deployment, but that is not verified.

Rust acceptance tests must cover complete/fragmented policy requests, trailing NUL, invalid requests, timeouts, graceful flushing, correct HTTP MIME type, and client connection using the policy. Use configurable explicit domains and ports, documenting the narrowing from upstream. Bind an unprivileged internal port and map host 843 rather than running the whole service as root. No client policy handshake has been captured yet.
