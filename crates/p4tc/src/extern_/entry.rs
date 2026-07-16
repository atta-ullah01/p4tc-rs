#[derive(Debug, Clone)]
pub struct ExternEntry {
    pub kind: String,
    pub instance: String,
    pub key: String,
    pub ext_id: u32,
    pub inst_id: u32,
    pub params: Vec<crate::table::Param>,
}
