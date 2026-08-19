use std::collections::HashMap;
use std::fmt;

/// A decoded key field value.
#[derive(Debug, Clone)]
pub enum DecodedValue {
    Ipv4(String),
    Ipv6(String),
    Mac(String),
    Int(u64),
    Raw(Vec<u8>),
}

impl fmt::Display for DecodedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ipv4(s) | Self::Ipv6(s) | Self::Mac(s) => write!(f, "{s}"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Raw(v) => {
                for b in v {
                    write!(f, "{b:02x}")?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: Vec<u8>,
    pub size: usize,
    pub type_name: String,
}

impl Param {
    /// Return the decoded value as a typed enum variant.
    ///
    /// - dev      -> Int (ifindex as u32)
    /// - ipv4     -> Ipv4 (dotted notation)
    /// - ipv6     -> Ipv6 (colon notation)
    /// - macaddr  -> Mac (colon-hex)
    /// - anything else -> Raw bytes
    pub fn decoded(&self) -> DecodedValue {
        match self.type_name.to_lowercase().as_str() {
            "dev" => {
                let mut buf = [0u8; 4];
                let len = self.value.len().min(4);
                buf[..len].copy_from_slice(&self.value[..len]);
                DecodedValue::Int(u32::from_le_bytes(buf) as u64)
            }
            "ipv4" if self.value.len() >= 4 => {
                DecodedValue::Ipv4(format!("{}.{}.{}.{}",
                    self.value[0], self.value[1],
                    self.value[2], self.value[3]))
            }
            "ipv6" if self.value.len() >= 16 => {
                let groups: Vec<String> = self.value[..16].chunks(2)
                    .map(|c| format!("{:02x}{:02x}", c[0], c.get(1).copied().unwrap_or(0)))
                    .collect();
                DecodedValue::Ipv6(groups.join(":"))
            }
            "macaddr" if self.value.len() >= 6 => {
                DecodedValue::Mac(format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    self.value[0], self.value[1], self.value[2],
                    self.value[3], self.value[4], self.value[5]))
            }
            _ => DecodedValue::Raw(self.value.clone()),
        }
    }

    /// Format the raw param bytes into a human-readable string.
    pub fn display_value(&self) -> String {
        self.decoded().to_string()
    }
}

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.name, self.display_value())
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub index: u32,
    pub params: Vec<Param>,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.params.is_empty() {
            write!(f, "(")?;
            for (i, p) in self.params.iter().enumerate() {
                if i > 0 { write!(f, ", ")?; }
                write!(f, "{}={}", p.name, p.display_value())?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TableEntry {
    pub table_name: String,
    pub priority: u32,
    /// Raw key bytes (for advanced use).
    pub key: Vec<u8>,
    pub key_size: u32,
    /// Decoded key fields, maps field name to typed value.
    /// Populated when a schema is loaded.
    pub key_fields: HashMap<String, DecodedValue>,
    pub mask: Option<Vec<u8>>,
    pub permissions: u32,
    pub dynamic: bool,
    pub aging_ms: u32,
    pub actions: Vec<Action>,
}

impl fmt::Display for TableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TableEntry(table={}, key={{", self.table_name)?;
        for (i, (name, val)) in self.key_fields.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{name}: {val}")?;
        }
        write!(f, "}}, prio={})", self.priority)
    }
}
