//! Bounded Engine.IO v3 text polling and a small Socket.IO v4 text subset.
//! This codec does not implement sessions, networking, authentication or timers.

use std::fmt;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_payload_bytes: usize,
    pub max_packet_bytes: usize,
    pub max_packets: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_payload_bytes: 64 * 1024,
            max_packet_bytes: 16 * 1024,
            max_packets: 64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    PayloadTooLarge,
    PacketTooLarge,
    TooManyPackets,
    InvalidLength,
    Truncated,
    SplitSurrogate,
    InvalidEnginePacket,
    InvalidSocketPacket,
    UnsupportedSocketPacket,
    InvalidNamespace,
    InvalidEvent,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PayloadTooLarge => "polling payload exceeds byte limit",
            Self::PacketTooLarge => "packet exceeds byte limit",
            Self::TooManyPackets => "polling payload exceeds packet count limit",
            Self::InvalidLength => "invalid Engine.IO v3 length prefix",
            Self::Truncated => "truncated Engine.IO v3 polling packet",
            Self::SplitSurrogate => "length prefix splits a UTF-16 surrogate pair",
            Self::InvalidEnginePacket => "invalid Engine.IO text packet kind",
            Self::InvalidSocketPacket => "invalid Socket.IO packet",
            Self::UnsupportedSocketPacket => {
                "Socket.IO acknowledgements or binary packets are unsupported"
            }
            Self::InvalidNamespace => "invalid Socket.IO namespace",
            Self::InvalidEvent => {
                "Socket.IO event must be a nonempty JSON array starting with a string"
            }
        })
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    Open,
    Close,
    Ping,
    Pong,
    Message,
    Upgrade,
    Noop,
}

impl EngineKind {
    fn marker(self) -> char {
        match self {
            Self::Open => '0',
            Self::Close => '1',
            Self::Ping => '2',
            Self::Pong => '3',
            Self::Message => '4',
            Self::Upgrade => '5',
            Self::Noop => '6',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnginePacket {
    pub kind: EngineKind,
    pub data: String,
}

pub fn decode_packet(text: &str, limits: Limits) -> Result<EnginePacket, Error> {
    check_packet_size(text.len(), limits)?;
    let kind = match text.as_bytes().first() {
        Some(b'0') => EngineKind::Open,
        Some(b'1') => EngineKind::Close,
        Some(b'2') => EngineKind::Ping,
        Some(b'3') => EngineKind::Pong,
        Some(b'4') => EngineKind::Message,
        Some(b'5') => EngineKind::Upgrade,
        Some(b'6') => EngineKind::Noop,
        _ => return Err(Error::InvalidEnginePacket),
    };
    Ok(EnginePacket {
        kind,
        data: text[1..].to_owned(),
    })
}

pub fn encode_packet(packet: &EnginePacket, limits: Limits) -> Result<String, Error> {
    let size = packet
        .data
        .len()
        .checked_add(1)
        .ok_or(Error::PacketTooLarge)?;
    check_packet_size(size, limits)?;
    let mut encoded = String::with_capacity(size);
    encoded.push(packet.kind.marker());
    encoded.push_str(&packet.data);
    Ok(encoded)
}

/// Decode text polling frames. Lengths count JavaScript UTF-16 code units,
/// not UTF-8 bytes or Unicode scalar values. Input must already be valid UTF-8.
pub fn decode_payload(text: &str, limits: Limits) -> Result<Vec<EnginePacket>, Error> {
    if text.len() > limits.max_payload_bytes {
        return Err(Error::PayloadTooLarge);
    }
    if text == "0:" {
        return Ok(Vec::new());
    }
    if text.is_empty() {
        return Err(Error::InvalidLength);
    }
    let mut remaining = text;
    let mut packets = Vec::new();
    while !remaining.is_empty() {
        if packets.len() >= limits.max_packets {
            return Err(Error::TooManyPackets);
        }
        let colon = remaining.find(':').ok_or(Error::InvalidLength)?;
        let prefix = &remaining[..colon];
        if prefix.is_empty() || prefix.len() > 20 || !prefix.bytes().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidLength);
        }
        let units = prefix.parse::<usize>().map_err(|_| Error::InvalidLength)?;
        if units == 0 {
            return Err(Error::InvalidLength);
        }
        // A valid UTF-8 packet always needs at least this many bytes.
        if units > limits.max_packet_bytes {
            return Err(Error::PacketTooLarge);
        }
        let body = &remaining[colon + 1..];
        let mut seen = 0;
        let mut boundary = None;
        for (offset, ch) in body.char_indices() {
            seen += ch.len_utf16();
            if seen > units {
                return Err(Error::SplitSurrogate);
            }
            if seen == units {
                boundary = Some(offset + ch.len_utf8());
                break;
            }
        }
        let boundary = boundary.ok_or(Error::Truncated)?;
        packets.push(decode_packet(&body[..boundary], limits)?);
        remaining = &body[boundary..];
    }
    Ok(packets)
}

pub fn encode_payload(packets: &[EnginePacket], limits: Limits) -> Result<String, Error> {
    if packets.len() > limits.max_packets {
        return Err(Error::TooManyPackets);
    }
    if packets.is_empty() {
        return if limits.max_payload_bytes >= 2 {
            Ok("0:".to_owned())
        } else {
            Err(Error::PayloadTooLarge)
        };
    }
    let mut output = String::new();
    for packet in packets {
        let encoded = encode_packet(packet, limits)?;
        let prefix = encoded.encode_utf16().count().to_string();
        let size = output
            .len()
            .checked_add(prefix.len())
            .and_then(|n| n.checked_add(1))
            .and_then(|n| n.checked_add(encoded.len()))
            .ok_or(Error::PayloadTooLarge)?;
        if size > limits.max_payload_bytes {
            return Err(Error::PayloadTooLarge);
        }
        output.push_str(&prefix);
        output.push(':');
        output.push_str(&encoded);
    }
    Ok(output)
}

#[derive(Clone, Debug, PartialEq)]
pub enum SocketPacket {
    Connect {
        namespace: String,
    },
    Disconnect {
        namespace: String,
    },
    Event {
        namespace: String,
        name: String,
        args: Vec<serde_json::Value>,
    },
}

/// Parse a Socket.IO v4 packet, excluding its Engine.IO message marker `4`.
/// ACK, binary, ERROR and CONNECT payloads are intentionally unsupported.
pub fn decode_socket(text: &str, limits: Limits) -> Result<SocketPacket, Error> {
    check_packet_size(text.len(), limits)?;
    let (kind, mut rest) = match text.as_bytes().first() {
        Some(kind @ (b'0' | b'1' | b'2')) => (*kind, &text[1..]),
        Some(b'3'..=b'6') => return Err(Error::UnsupportedSocketPacket),
        _ => return Err(Error::InvalidSocketPacket),
    };
    let namespace = if rest.starts_with('/') {
        let end = rest.find(',').unwrap_or(rest.len());
        let namespace = &rest[..end];
        validate_namespace(namespace)?;
        rest = if end == rest.len() {
            ""
        } else {
            &rest[end + 1..]
        };
        namespace.to_owned()
    } else {
        "/".to_owned()
    };
    if rest.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return Err(Error::UnsupportedSocketPacket);
    }
    match kind {
        b'0' if rest.is_empty() => Ok(SocketPacket::Connect { namespace }),
        b'1' if rest.is_empty() => Ok(SocketPacket::Disconnect { namespace }),
        b'2' => {
            let value: serde_json::Value =
                serde_json::from_str(rest).map_err(|_| Error::InvalidEvent)?;
            let serde_json::Value::Array(mut values) = value else {
                return Err(Error::InvalidEvent);
            };
            if !values.first().is_some_and(serde_json::Value::is_string) {
                return Err(Error::InvalidEvent);
            }
            let name = values
                .remove(0)
                .as_str()
                .ok_or(Error::InvalidEvent)?
                .to_owned();
            Ok(SocketPacket::Event {
                namespace,
                name,
                args: values,
            })
        }
        _ => Err(Error::InvalidSocketPacket),
    }
}

pub fn encode_socket(packet: &SocketPacket, limits: Limits) -> Result<String, Error> {
    let (kind, namespace) = match packet {
        SocketPacket::Connect { namespace } => ('0', namespace),
        SocketPacket::Disconnect { namespace } => ('1', namespace),
        SocketPacket::Event { namespace, .. } => ('2', namespace),
    };
    validate_namespace(namespace)?;
    let mut output = String::from(kind);
    if namespace != "/" {
        check_packet_size(namespace.len().saturating_add(2), limits)?;
        output.push_str(namespace);
        output.push(',');
    }
    if let SocketPacket::Event { name, args, .. } = packet {
        // A bounded writer prevents serde from allocating an oversized output.
        // serde's serializing path is recursive, so reject excessive caller-built depth.
        if args.iter().any(|v| json_depth(v, 0) > 64) {
            return Err(Error::InvalidEvent);
        }
        let mut writer = BoundedWriter {
            bytes: output.into_bytes(),
            max: limits.max_packet_bytes,
        };
        use std::io::Write;
        writer.write_all(b"[").map_err(|_| Error::PacketTooLarge)?;
        serde_json::to_writer(&mut writer, name).map_err(|_| Error::PacketTooLarge)?;
        for arg in args {
            writer.write_all(b",").map_err(|_| Error::PacketTooLarge)?;
            serde_json::to_writer(&mut writer, arg).map_err(|_| Error::PacketTooLarge)?;
        }
        writer.write_all(b"]").map_err(|_| Error::PacketTooLarge)?;
        output = String::from_utf8(writer.bytes).map_err(|_| Error::InvalidEvent)?;
    }
    check_packet_size(output.len(), limits)?;
    Ok(output)
}

fn check_packet_size(size: usize, limits: Limits) -> Result<(), Error> {
    if size > limits.max_packet_bytes {
        Err(Error::PacketTooLarge)
    } else {
        Ok(())
    }
}

fn validate_namespace(namespace: &str) -> Result<(), Error> {
    if !namespace.starts_with('/') || namespace.chars().any(|c| c == ',' || c.is_control()) {
        Err(Error::InvalidNamespace)
    } else {
        Ok(())
    }
}

fn json_depth(value: &serde_json::Value, depth: usize) -> usize {
    if depth > 64 {
        return depth;
    }
    match value {
        serde_json::Value::Array(values) => values
            .iter()
            .map(|v| json_depth(v, depth + 1))
            .max()
            .unwrap_or(depth),
        serde_json::Value::Object(values) => values
            .values()
            .map(|v| json_depth(v, depth + 1))
            .max()
            .unwrap_or(depth),
        _ => depth,
    }
}

struct BoundedWriter {
    bytes: Vec<u8>,
    max: usize,
}

impl std::io::Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.max.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("packet too large"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
