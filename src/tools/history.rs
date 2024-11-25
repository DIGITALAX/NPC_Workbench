use serde_json::Value;

use crate::workflow::ExecutionHistory;

#[derive(Clone, Debug)]
pub enum HistoryParse {
    ExtractField {
        index: usize,             
        field_path: Vec<String>, 
    },
    CustomProcessor {
        function: fn(Vec<ExecutionHistory>) -> Result<Value, String>,
    },
}

impl HistoryParse {
    pub fn process(&self, history: Vec<ExecutionHistory>) -> Result<Value, String> {
        match self {
            HistoryParse::ExtractField { index, field_path } => {
                if let Some(entry) = history.get(*index) {
                    if let Some(result) = &entry.result {
                        let mut current_value = result;

                        for key in field_path {
                            match current_value {
                                Value::Object(map) => {
                                    current_value = map.get(key).ok_or_else(|| {
                                        format!(
                                            "Field '{}' not found in result at index {}",
                                            key, index
                                        )
                                    })?;
                                }
                                _ => {
                                    return Err(format!(
                                        "Invalid field path: '{}' in result at index {}",
                                        key, index
                                    ))
                                }
                            }
                        }

                        return Ok(current_value.clone());
                    } else {
                        Err(format!(
                            "No result found in ExecutionHistory at index {}",
                            index
                        ))
                    }
                } else {
                    Err(format!(
                        "Index {} out of bounds in ExecutionHistory (length: {})",
                        index,
                        history.len()
                    ))
                }
            }

            HistoryParse::CustomProcessor { function } => function(history),
        }
    }
}
