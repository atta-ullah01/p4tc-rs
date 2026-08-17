use std::fmt;

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: Vec<u8>,
    pub size: usize,
    pub type_name: String,
}

impl Param {
    /// Format the raw param bytes into something readable based on its type.
    ///
    /// For dev params, we decode the ifindex as a plain decimal integer.
    /// For ipv4/ipv6, we use standard dotted or colon notation.
    /// For macaddr, we print the usual colon-separated hex.
    /// Anything else just comes out as raw lowercase hex.
    pub fn display_value(&self) -> String {
        match self.type_name.to_lowercase().as_str() {
            "dev" => {
                // the library gives us the ifindex as a little-endian u32
                let mut buf = [0u8; 4];
                let len = self.value.len().min(4);
                buf[..len].copy_from_slice(&self.value[..len]);
                u32::from_le_bytes(buf).to_string()
            }
            "ipv4" if self.value.len() >= 4 => {
                format!("{}.{}.{}.{}", self.value[0], self.value[1],
                        self.value[2], self.value[3])
            }
            "ipv6" if self.value.len() >= 16 => {
                let groups: Vec<String> = self.value.chunks(2)
                    .map(|c| format!("{:02x}{:02x}", c[0], c.get(1).copied().unwrap_or(0)))
                    .collect();
                groups.join(":")
            }
            "macaddr" if self.value.len() >= 6 => {
                format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        self.value[0], self.value[1], self.value[2],
                        self.value[3], self.value[4], self.value[5])
            }
            _ => self.value.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        }
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

#[derive(Debug, Clone)]
pub struct TableEntry {
    pub table_name: String,
    pub priority: u32,
    pub key: Vec<u8>,
    pub key_size: u32,
    pub mask: Option<Vec<u8>>,
    pub permissions: u32,
    pub dynamic: bool,
    pub aging_ms: u32,
    pub actions: Vec<Action>,
}
