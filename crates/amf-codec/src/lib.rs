//! Bounded AMF envelope proof of concept. See README for compatibility limits.
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    Integer(i32),
    String(String),
    ByteArray(Vec<u8>),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
    MixedArray(Vec<(String, Value)>),
    Amf3(Box<Value>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub name: String,
    pub required: bool,
    pub data: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub target: String,
    pub response: String,
    pub data: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Envelope {
    pub version: u16,
    pub headers: Vec<Header>,
    pub bodies: Vec<Body>,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_depth: usize,
    pub max_values: usize,
    pub max_string_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 1024 * 1024,
            max_depth: 32,
            max_values: 100_000,
            max_string_bytes: 256 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub &'static str);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for Error {}
type Result<T> = std::result::Result<T, Error>;

struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
    limits: Limits,
    values: usize,
    decoded_string_bytes: usize,
    strings: Vec<String>,
    objects0: Vec<Option<CachedValue>>,
    objects3: Vec<Option<CachedValue>>,
    traits: Vec<Trait>,
}

struct CachedValue {
    value: Value,
    nodes: usize,
    string_bytes: usize,
    height: usize,
}

#[derive(Clone)]
struct Trait {
    keys: Vec<String>,
    dynamic: bool,
}

fn value_size(value: &Value) -> (usize, usize, usize) {
    let (mut nodes, mut strings, mut height) = (1usize, 0usize, 0usize);
    let mut child = |v: &Value| {
        let (n, s, h) = value_size(v);
        nodes = nodes.saturating_add(n);
        strings = strings.saturating_add(s);
        height = height.max(h.saturating_add(1));
    };
    match value {
        Value::Array(values) => values.iter().for_each(&mut child),
        Value::Object(fields) | Value::MixedArray(fields) => {
            for (_, value) in fields {
                child(value);
            }
            for (key, _) in fields {
                strings = strings.saturating_add(key.len());
            }
        }
        Value::Amf3(value) => child(value),
        Value::String(value) => strings = value.len(),
        Value::ByteArray(value) => strings = value.len(),
        _ => {}
    }
    (nodes, strings, height)
}

impl Reader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.pos.checked_add(n).ok_or(Error("length overflow"))?;
        let b = self
            .input
            .get(self.pos..end)
            .ok_or(Error("truncated input"))?;
        self.pos = end;
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.bytes(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
    }
    fn number(&mut self) -> Result<f64> {
        Ok(f64::from_be_bytes(self.bytes(8)?.try_into().unwrap()))
    }
    fn string(&mut self, n: usize) -> Result<String> {
        if n > self.limits.max_string_bytes {
            return Err(Error("string limit exceeded"));
        }
        self.charge_string(n)?;
        String::from_utf8(self.bytes(n)?.to_vec()).map_err(|_| Error("invalid UTF-8"))
    }
    fn charge_string(&mut self, n: usize) -> Result<()> {
        if n > self
            .limits
            .max_bytes
            .saturating_sub(self.decoded_string_bytes)
        {
            return Err(Error("decoded string budget exceeded"));
        }
        self.decoded_string_bytes += n;
        Ok(())
    }
    fn short_string(&mut self) -> Result<String> {
        let n = self.u16()? as usize;
        self.string(n)
    }
    fn u29(&mut self) -> Result<u32> {
        let mut value = 0;
        for _ in 0..3 {
            let b = self.byte()?;
            value = (value << 7) | u32::from(b & 127);
            if b & 128 == 0 {
                return Ok(value);
            }
        }
        Ok((value << 8) | u32::from(self.byte()?))
    }
    fn string3(&mut self) -> Result<String> {
        let info = self.u29()?;
        if info & 1 == 0 {
            let index = (info >> 1) as usize;
            let size = self
                .strings
                .get(index)
                .ok_or(Error("invalid string reference"))?
                .len();
            self.charge_string(size)?;
            return Ok(self.strings[index].clone());
        }
        let s = self.string((info >> 1) as usize)?;
        if !s.is_empty() {
            self.strings.push(s.clone());
        }
        Ok(s)
    }
    fn budget(&mut self, depth: usize) -> Result<()> {
        self.charge_values(1, depth)
    }
    fn charge_values(&mut self, count: usize, depth: usize) -> Result<()> {
        if depth > self.limits.max_depth
            || count > self.limits.max_values.saturating_sub(self.values)
        {
            return Err(Error("value or depth limit exceeded"));
        }
        self.values += count;
        Ok(())
    }
    fn reserve_object(&mut self, amf3: bool) -> usize {
        let objects = if amf3 {
            &mut self.objects3
        } else {
            &mut self.objects0
        };
        let index = objects.len();
        objects.push(None);
        index
    }
    fn finish_object(&mut self, amf3: bool, index: usize, value: Value) -> Result<Value> {
        let (nodes, string_bytes, height) = value_size(&value);
        // Cache copies also consume budgets, so nested objects cannot bypass allocation limits.
        self.charge_values(nodes, height)?;
        self.charge_string(string_bytes)?;
        let cached = CachedValue {
            value: value.clone(),
            nodes,
            string_bytes,
            height,
        };
        let objects = if amf3 {
            &mut self.objects3
        } else {
            &mut self.objects0
        };
        objects[index] = Some(cached);
        Ok(value)
    }
    fn reference(&mut self, amf3: bool, index: usize, depth: usize) -> Result<Value> {
        let objects = if amf3 { &self.objects3 } else { &self.objects0 };
        let cached = objects
            .get(index)
            .ok_or(Error("invalid object reference"))?
            .as_ref()
            .ok_or(Error("cyclic object reference not supported"))?;
        let (nodes, strings, height) = (cached.nodes, cached.string_bytes, cached.height);
        // The root was charged by value0/value3 already.
        self.charge_values(nodes - 1, depth.saturating_add(height))?;
        self.charge_string(strings)?;
        let objects = if amf3 { &self.objects3 } else { &self.objects0 };
        Ok(objects[index].as_ref().unwrap().value.clone())
    }
    fn properties(&mut self, depth: usize) -> Result<Vec<(String, Value)>> {
        let mut fields = Vec::new();
        loop {
            let key = self.short_string()?;
            if key.is_empty() && self.input.get(self.pos) == Some(&9) {
                self.pos += 1;
                break;
            }
            fields.push((key, self.value0(depth + 1)?));
        }
        Ok(fields)
    }
    fn value0(&mut self, depth: usize) -> Result<Value> {
        self.budget(depth)?;
        Ok(match self.byte()? {
            0 => Value::Number(self.number()?),
            1 => Value::Bool(self.byte()? != 0),
            2 => Value::String(self.short_string()?),
            3 => {
                let index = self.reserve_object(false);
                let value = Value::Object(self.properties(depth)?);
                self.finish_object(false, index, value)?
            }
            5 => Value::Null,
            6 => Value::Undefined,
            7 => {
                let index = self.u16()? as usize;
                self.reference(false, index, depth)?
            }
            8 => {
                self.u32()?;
                let index = self.reserve_object(false);
                let value = Value::MixedArray(self.properties(depth)?);
                self.finish_object(false, index, value)?
            }
            10 => {
                let n = self.u32()? as usize;
                if n > self.limits.max_values.saturating_sub(self.values) {
                    return Err(Error("array limit exceeded"));
                }
                let mut values = Vec::new();
                let index = self.reserve_object(false);
                for _ in 0..n {
                    values.push(self.value0(depth + 1)?);
                }
                self.finish_object(false, index, Value::Array(values))?
            }
            12 => {
                let n = self.u32()? as usize;
                Value::String(self.string(n)?)
            }
            // Flash Player serialises ActionScript values that have no AMF0
            // representation (e.g. Dictionary) with the 0x0D Unsupported
            // marker; the AIR client sends it and SabreAMF tolerated it.
            13 => Value::Null,
            17 => {
                self.strings.clear();
                self.objects3.clear();
                self.traits.clear();
                Value::Amf3(Box::new(self.value3(depth + 1)?))
            }
            _ => return Err(Error("unsupported AMF0 marker")),
        })
    }
    fn value3(&mut self, depth: usize) -> Result<Value> {
        self.budget(depth)?;
        Ok(match self.byte()? {
            0 => Value::Undefined,
            1 => Value::Null,
            2 => Value::Bool(false),
            3 => Value::Bool(true),
            4 => {
                let n = self.u29()?;
                Value::Integer(if n & 0x10000000 != 0 {
                    (n | 0xe0000000) as i32
                } else {
                    n as i32
                })
            }
            5 => Value::Number(self.number()?),
            6 => Value::String(self.string3()?),
            12 => {
                let info = self.u29()?;
                if info & 1 == 0 {
                    return self.reference(true, (info >> 1) as usize, depth);
                }
                let count = (info >> 1) as usize;
                self.charge_string(count)?;
                let index = self.reserve_object(true);
                let value = Value::ByteArray(self.bytes(count)?.to_vec());
                self.finish_object(true, index, value)?
            }
            9 => {
                let info = self.u29()?;
                if info & 1 == 0 {
                    return self.reference(true, (info >> 1) as usize, depth);
                }
                let n = (info >> 1) as usize;
                if n > self.limits.max_values.saturating_sub(self.values) {
                    return Err(Error("array limit exceeded"));
                }
                let mut fields = Vec::new();
                let index = self.reserve_object(true);
                loop {
                    let key = self.string3()?;
                    if key.is_empty() {
                        break;
                    }
                    fields.push((key, self.value3(depth + 1)?));
                }
                let mut values = Vec::new();
                for _ in 0..n {
                    values.push(self.value3(depth + 1)?);
                }
                let value = if fields.is_empty() {
                    Value::Array(values)
                } else if values.is_empty() {
                    Value::MixedArray(fields)
                } else {
                    return Err(Error(
                        "mixed dense and associative AMF3 arrays not supported",
                    ));
                };
                self.finish_object(true, index, value)?
            }
            10 => {
                let info = self.u29()?;
                if info & 1 == 0 {
                    return self.reference(true, (info >> 1) as usize, depth);
                }
                let object_trait = if info & 2 == 0 {
                    let index = (info >> 2) as usize;
                    let stored = self
                        .traits
                        .get(index)
                        .ok_or(Error("invalid trait reference"))?;
                    let bytes = stored.keys.iter().map(String::len).sum();
                    self.charge_string(bytes)?;
                    self.traits[index].clone()
                } else {
                    if info & 4 != 0 {
                        return Err(Error("externalizable objects not supported"));
                    }
                    let class = self.string3()?;
                    if !class.is_empty() {
                        return Err(Error("typed objects not supported by spike"));
                    }
                    let count = (info >> 4) as usize;
                    if count > self.limits.max_values.saturating_sub(self.values) {
                        return Err(Error("property limit exceeded"));
                    }
                    let mut keys = Vec::new();
                    for _ in 0..count {
                        keys.push(self.string3()?);
                    }
                    self.charge_string(keys.iter().map(String::len).sum())?;
                    let object_trait = Trait {
                        keys,
                        dynamic: info & 8 != 0,
                    };
                    self.traits.push(object_trait.clone());
                    object_trait
                };
                let index = self.reserve_object(true);
                let mut fields = Vec::new();
                for key in object_trait.keys {
                    fields.push((key, self.value3(depth + 1)?));
                }
                if object_trait.dynamic {
                    loop {
                        let key = self.string3()?;
                        if key.is_empty() {
                            break;
                        }
                        fields.push((key, self.value3(depth + 1)?));
                    }
                }
                self.finish_object(true, index, Value::Object(fields))?
            }
            _ => return Err(Error("unsupported AMF3 marker")),
        })
    }
    fn section(&mut self) -> Result<Value> {
        let size = self.u32()?;
        let start = self.pos;
        self.objects0.clear();
        let value = self.value0(0)?;
        if size != u32::MAX && self.pos - start != size as usize {
            return Err(Error("declared length mismatch"));
        }
        Ok(value)
    }
}

pub fn decode(input: &[u8], limits: Limits) -> Result<Envelope> {
    if input.len() > limits.max_bytes {
        return Err(Error("message limit exceeded"));
    }
    let mut r = Reader {
        input,
        pos: 0,
        limits,
        values: 0,
        decoded_string_bytes: 0,
        strings: Vec::new(),
        objects0: Vec::new(),
        objects3: Vec::new(),
        traits: Vec::new(),
    };
    let version = r.u16()?;
    if version != 0 && version != 3 {
        return Err(Error("unsupported envelope version"));
    }
    let mut headers = Vec::new();
    for _ in 0..r.u16()? {
        let name = r.short_string()?;
        let required = r.byte()? != 0;
        headers.push(Header {
            name,
            required,
            data: r.section()?,
        });
    }
    let mut bodies = Vec::new();
    for _ in 0..r.u16()? {
        let target = r.short_string()?;
        let response = r.short_string()?;
        bodies.push(Body {
            target,
            response,
            data: r.section()?,
        });
    }
    if r.pos != input.len() {
        return Err(Error("trailing input"));
    }
    Ok(Envelope {
        version,
        headers,
        bodies,
    })
}

struct Writer {
    bytes: Vec<u8>,
    limits: Limits,
    values: usize,
}
impl Writer {
    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > self.limits.max_bytes.saturating_sub(self.bytes.len()) {
            return Err(Error("message limit exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn byte(&mut self, value: u8) -> Result<()> {
        self.put(&[value])
    }
    fn short(&mut self, n: usize) -> Result<()> {
        self.put(
            &u16::try_from(n)
                .map_err(|_| Error("u16 length overflow"))?
                .to_be_bytes(),
        )
    }
    fn string0(&mut self, value: &str) -> Result<()> {
        self.check_string(value)?;
        self.short(value.len())?;
        self.put(value.as_bytes())
    }
    fn check_string(&self, value: &str) -> Result<()> {
        if value.len() > self.limits.max_string_bytes {
            Err(Error("string limit exceeded"))
        } else {
            Ok(())
        }
    }
    fn u29(&mut self, n: u32) -> Result<()> {
        if n > 0x1fffffff {
            return Err(Error("u29 overflow"));
        }
        if n < 128 {
            self.byte(n as u8)
        } else if n < 16384 {
            self.put(&[((n >> 7) as u8) | 128, (n & 127) as u8])
        } else if n < 2097152 {
            self.put(&[
                ((n >> 14) as u8) | 128,
                ((n >> 7) as u8) | 128,
                (n & 127) as u8,
            ])
        } else {
            self.put(&[
                ((n >> 22) as u8) | 128,
                ((n >> 15) as u8) | 128,
                ((n >> 8) as u8) | 128,
                n as u8,
            ])
        }
    }
    fn string3(&mut self, s: &str) -> Result<()> {
        self.check_string(s)?;
        if s.len() > 0xfffffff {
            return Err(Error("string length overflow"));
        }
        self.u29(((s.len() as u32) << 1) | 1)?;
        self.put(s.as_bytes())
    }
    fn value(&mut self, v: &Value, amf3: bool, depth: usize) -> Result<()> {
        self.values += 1;
        if depth > self.limits.max_depth || self.values > self.limits.max_values {
            return Err(Error("value or depth limit exceeded"));
        }
        if amf3 {
            return self.value3(v, depth);
        }
        match v {
            Value::ByteArray(_) => {
                self.byte(17)?;
                if depth >= self.limits.max_depth {
                    return Err(Error("value or depth limit exceeded"));
                }
                self.value3(v, depth + 1)
            }
            Value::Undefined => self.byte(6),
            Value::Null => self.byte(5),
            Value::Bool(v) => {
                self.byte(1)?;
                self.byte(u8::from(*v))
            }
            Value::Number(v) => {
                self.byte(0)?;
                self.put(&v.to_be_bytes())
            }
            Value::Integer(v) => {
                self.byte(0)?;
                self.put(&f64::from(*v).to_be_bytes())
            }
            Value::String(v) => {
                self.check_string(v)?;
                if v.len() <= u16::MAX as usize {
                    self.byte(2)?;
                    self.string0(v)
                } else {
                    self.byte(12)?;
                    self.put(&(v.len() as u32).to_be_bytes())?;
                    self.put(v.as_bytes())
                }
            }
            Value::Array(v) => {
                self.byte(10)?;
                self.put(
                    &u32::try_from(v.len())
                        .map_err(|_| Error("array length overflow"))?
                        .to_be_bytes(),
                )?;
                for v in v {
                    self.value(v, false, depth + 1)?;
                }
                Ok(())
            }
            Value::Object(fields) | Value::MixedArray(fields) => {
                if matches!(v, Value::MixedArray(_)) {
                    self.byte(8)?;
                    self.put(&[0; 4])?;
                } else {
                    self.byte(3)?;
                }
                for (key, value) in fields {
                    self.string0(key)?;
                    self.value(value, false, depth + 1)?;
                }
                self.put(&[0, 0, 9])
            }
            Value::Amf3(value) => {
                self.byte(17)?;
                self.value(value, true, depth + 1)
            }
        }
    }
    fn value3(&mut self, v: &Value, depth: usize) -> Result<()> {
        match v {
            Value::ByteArray(bytes) => {
                if bytes.len() > 0xfffffff || bytes.len() > self.limits.max_bytes {
                    return Err(Error("byte array limit exceeded"));
                }
                self.byte(12)?;
                self.u29(((bytes.len() as u32) << 1) | 1)?;
                self.put(bytes)
            }
            Value::Undefined => self.byte(0),
            Value::Null => self.byte(1),
            Value::Bool(v) => self.byte(if *v { 3 } else { 2 }),
            Value::Integer(v) if (-268435456..=268435455).contains(v) => {
                self.byte(4)?;
                self.u29((*v as u32) & 0x1fffffff)
            }
            Value::Integer(v) => {
                self.byte(5)?;
                self.put(&f64::from(*v).to_be_bytes())
            }
            Value::Number(v) => {
                self.byte(5)?;
                self.put(&v.to_be_bytes())
            }
            Value::String(v) => {
                self.byte(6)?;
                self.string3(v)
            }
            Value::Array(values) => {
                if values.len() > 0xfffffff {
                    return Err(Error("array length overflow"));
                }
                self.byte(9)?;
                self.u29(((values.len() as u32) << 1) | 1)?;
                self.string3("")?;
                for v in values {
                    self.value(v, true, depth + 1)?;
                }
                Ok(())
            }
            Value::MixedArray(fields) => {
                self.byte(9)?;
                self.u29(1)?;
                for (key, v) in fields {
                    if key.is_empty() {
                        return Err(Error("empty associative key"));
                    }
                    self.string3(key)?;
                    self.value(v, true, depth + 1)?;
                }
                self.string3("")
            }
            Value::Object(fields) => {
                if fields.len() > 0x1ffffff {
                    return Err(Error("property length overflow"));
                }
                self.byte(10)?;
                self.u29(((fields.len() as u32) << 4) | 3)?;
                self.string3("")?;
                for (key, _) in fields {
                    self.string3(key)?;
                }
                for (_, v) in fields {
                    self.value(v, true, depth + 1)?;
                }
                Ok(())
            }
            Value::Amf3(_) => Err(Error("nested AMF3 wrapper")),
        }
    }
    fn section(&mut self, v: &Value) -> Result<()> {
        self.put(&u32::MAX.to_be_bytes())?;
        self.value(v, false, 0)
    }
}

/// Writes unknown section lengths, matching SabreAMF_Message::serialize.
/// Version 3 is envelope metadata; callers explicitly wrap body values in `Amf3`.
pub fn encode(envelope: &Envelope, limits: Limits) -> Result<Vec<u8>> {
    if envelope.version != 0 && envelope.version != 3 {
        return Err(Error("unsupported envelope version"));
    }
    let mut w = Writer {
        bytes: Vec::new(),
        limits,
        values: 0,
    };
    w.short(envelope.version as usize)?;
    w.short(envelope.headers.len())?;
    for h in &envelope.headers {
        w.string0(&h.name)?;
        w.byte(u8::from(h.required))?;
        w.section(&h.data)?;
    }
    w.short(envelope.bodies.len())?;
    for b in &envelope.bodies {
        w.string0(&b.target)?;
        w.string0(&b.response)?;
        w.section(&b.data)?;
    }
    Ok(w.bytes)
}
