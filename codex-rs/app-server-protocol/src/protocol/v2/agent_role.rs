use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AgentRoleToolSelectionCatalogReadParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum AgentRoleToolSelectionCatalogExposure {
    Direct,
    Deferred,
    DirectModelOnly,
    Hidden,
    Hosted,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AgentRoleToolSelectionCatalogEntry {
    pub name: String,
    pub selected: bool,
    pub exposure: AgentRoleToolSelectionCatalogExposure,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct AgentRoleToolSelectionCatalogReadResponse {
    pub data: Vec<AgentRoleToolSelectionCatalogEntry>,
    pub unmatched_allowed_tools: Vec<String>,
}
