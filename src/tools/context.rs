use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub enum ContextParse {
    ParseFields {
        expected_format: Map<String, Value>,
        required_fields: Vec<String>,
    },
    CustomProcessor {
        function: fn(Value) -> Result<Value, String>,
    },
}

impl ContextParse {
    pub fn process(&self, input: Value) -> Result<Value, String> {
        match self {
            ContextParse::ParseFields {
                expected_format,
                required_fields,
            } => {
                if let Value::Object(map) = input {
                    let missing_fields: Vec<String> = required_fields
                        .iter()
                        .filter(|key| !map.contains_key(*key))
                        .cloned()
                        .collect();

                    if !missing_fields.is_empty() {
                        return Err(format!("Required Field Not Found: {:?}", missing_fields));
                    }

                    let parsed_fields = map
                        .into_iter()
                        .filter(|(key, _)| expected_format.contains_key(key))
                        .collect::<Map<String, Value>>();

                    Ok(Value::Object(parsed_fields))
                } else {
                    Err("The input is not a valid JSON.".to_string())
                }
            }

            ContextParse::CustomProcessor { function } => function(input),
        }
    }
}
