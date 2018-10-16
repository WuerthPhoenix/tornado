#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operator {
    #[serde(rename = "AND")]
    And { operators: Vec<Operator> },
    #[serde(rename = "OR")]
    Or { operators: Vec<Operator> },
    #[serde(rename = "equals")]
    Equals { first: String, second: String },
    #[serde(rename = "regex")]
    Regex { regex: String, target: String },
}
