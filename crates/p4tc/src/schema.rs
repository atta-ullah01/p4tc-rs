use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ParamSchema {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub bitwidth: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionSchema {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub params: Vec<ParamSchema>,
}

impl ActionSchema {
    pub fn validate_params(&self, params: &HashMap<&str, &str>) -> Result<Vec<String>, String> {
        let schema_names: Vec<&str> = self.params.iter().map(|p| p.name.as_str()).collect();
        let unknown: Vec<&&str> = params.keys().filter(|k| !schema_names.contains(k)).collect();
        if !unknown.is_empty() {
            return Err(format!(
                "unknown param(s) {:?} for action '{}', available: {:?}",
                unknown, self.name, schema_names,
            ));
        }
        Ok(self.params.iter()
            .filter_map(|p| params.get(p.name.as_str()).map(|v| v.to_string()))
            .collect())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyFieldSchema {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default = "default_match_type")]
    pub match_type: String,
    pub bitwidth: u32,
}

fn default_match_type() -> String { "exact".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub id: u32,
    #[serde(default)]
    pub keysize: u32,
    #[serde(default, rename = "keyfields")]
    pub key_fields: Vec<KeyFieldSchema>,
    #[serde(default, deserialize_with = "actions_as_map")]
    pub actions: HashMap<String, ActionSchema>,
}

fn actions_as_map<'de, D>(de: D) -> Result<HashMap<String, ActionSchema>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Vec<ActionSchema> = Vec::deserialize(de)?;
    Ok(v.into_iter().map(|a| (a.name.clone(), a)).collect())
}

impl TableSchema {
    pub fn validate_key(&self, key: &HashMap<&str, &str>) -> Result<Vec<String>, String> {
        let schema_names: Vec<&str> = self.key_fields.iter().map(|f| f.name.as_str()).collect();
        let unknown: Vec<&&str> = key.keys().filter(|k| !schema_names.contains(k)).collect();
        if !unknown.is_empty() {
            return Err(format!(
                "unknown key field(s) {:?}, available: {:?}",
                unknown, schema_names,
            ));
        }
        Ok(self.key_fields.iter()
            .filter_map(|f| key.get(f.name.as_str()).map(|v| v.to_string()))
            .collect())
    }

    pub fn get_action(&self, name: &str) -> Option<&ActionSchema> {
        self.actions.get(name)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternInstanceSchema {
    #[serde(rename = "inst_name")]
    pub name: String,
    #[serde(default, rename = "inst_id")]
    pub id: u32,
    #[serde(default, deserialize_with = "data_param_names")]
    pub param_names: Vec<String>,
}

fn data_param_names<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct RawParam {
        name: String,
        #[serde(default)]
        attr: String,
    }
    let v: Vec<RawParam> = Vec::deserialize(de)?;
    Ok(v.into_iter().filter(|p| p.attr == "param").map(|p| p.name).collect())
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExternSchema {
    pub name: String,
    #[serde(default)]
    pub id: String,
    #[serde(default, deserialize_with = "instances_as_map")]
    pub instances: HashMap<String, ExternInstanceSchema>,
}

fn instances_as_map<'de, D>(de: D) -> Result<HashMap<String, ExternInstanceSchema>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Vec<ExternInstanceSchema> = Vec::deserialize(de)?;
    Ok(v.into_iter().map(|i| (i.name.clone(), i)).collect())
}

impl ExternSchema {
    pub fn get_instance(&self, name: &str) -> Option<&ExternInstanceSchema> {
        self.instances.get(name)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RawPipeline {
    #[serde(default)]
    tables: Vec<TableSchema>,
    #[serde(default)]
    externs: Vec<ExternSchema>,
}

#[derive(Debug, Clone)]
pub struct PipelineSchema {
    pub name: String,
    pub tables: HashMap<String, TableSchema>,
    pub externs: HashMap<String, ExternSchema>,
}

impl PipelineSchema {
    pub fn load(name: &str, template_path: Option<&str>) -> Option<Self> {
        let json_path = resolve_path(name, template_path)?;
        let data = std::fs::read_to_string(&json_path).ok()?;
        let raw: RawPipeline = serde_json::from_str(&data).ok()?;

        let tables = raw.tables.into_iter().map(|t| (t.name.clone(), t)).collect();
        let externs = raw.externs.into_iter().map(|e| (e.name.clone(), e)).collect();

        Some(Self { name: name.to_owned(), tables, externs })
    }

    pub fn get_table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.get(name)
    }

    pub fn get_extern(&self, name: &str) -> Option<&ExternSchema> {
        self.externs.get(name)
    }
}

fn resolve_path(name: &str, template_path: Option<&str>) -> Option<PathBuf> {
    let filename = format!("{name}.json");

    if let Some(dir) = template_path {
        let p = Path::new(dir).join(&filename);
        if p.exists() { return Some(p); }
    }

    if let Ok(dir) = std::env::var("INTROSPECTION") {
        let p = Path::new(&dir).join(&filename);
        if p.exists() { return Some(p); }
    }

    let p = PathBuf::from(&filename);
    if p.exists() { return Some(p); }

    None
}
