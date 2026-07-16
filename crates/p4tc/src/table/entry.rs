#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub value: Vec<u8>,
    pub size: usize,
    pub type_name: String,
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
