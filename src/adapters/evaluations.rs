use serde_json::{Map, Value};

use crate::{
    nibble::{Adaptable, Nibble},
    utils::generate_unique_id,
};

use std::{error::Error, sync::Arc};

#[derive(Debug, Clone)]
pub struct Evaluation {
    pub name: String,
    pub encrypted: bool,
    pub id: Vec<u8>,
    pub evaluation_type: EvaluationType,
}

#[derive(Clone)]
pub enum EvaluationType {
    HumanJudge {
        prompt: String,
        approval_required: bool,
    },
    LLMJudge {
        model_name: String,
        prompt_template: String,
        approval_threshold: f64,
    },
    ContextualJudge {
        context_fn: Arc<dyn Fn(Value) -> bool + Send + Sync>,
    },
}

impl std::fmt::Debug for EvaluationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvaluationType::HumanJudge {
                prompt,
                approval_required,
            } => f
                .debug_struct("HumanJudge")
                .field("prompt", prompt)
                .field("approval_required", approval_required)
                .finish(),
            EvaluationType::LLMJudge {
                model_name,
                prompt_template,
                approval_threshold,
            } => f
                .debug_struct("LLMJudge")
                .field("model_name", model_name)
                .field("prompt_template", prompt_template)
                .field("approval_threshold", approval_threshold)
                .finish(),
            EvaluationType::ContextualJudge { .. } => f
                .debug_struct("ContextualJudge")
                .field("context_fn", &"Function pointer")
                .finish(),
        }
    }
}

pub fn configure_new_evaluation(
    nibble: &mut Nibble,
    name: &str,
    evaluation_type: EvaluationType,
    encrypted: bool,
) -> Result<Evaluation, Box<dyn Error>> {
    let evaluation = Evaluation {
        name: name.to_string(),
        encrypted,
        id: generate_unique_id(),
        evaluation_type,
    };
    nibble.evaluations.push(evaluation.clone());
    Ok(evaluation)
}

impl Adaptable for Evaluation {
    fn name(&self) -> &str {
        &self.name
    }
    fn id(&self) -> &Vec<u8> {
        &self.id
    }
}

impl EvaluationType {
    pub fn to_json(&self) -> Value {
        match self {
            EvaluationType::HumanJudge {
                prompt,
                approval_required,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("HumanJudge".to_string()));
                map.insert("prompt".to_string(), Value::String(prompt.clone()));
                map.insert(
                    "approval_required".to_string(),
                    Value::Bool(*approval_required),
                );
                Value::Object(map)
            }
            EvaluationType::LLMJudge {
                model_name,
                prompt_template,
                approval_threshold,
            } => {
                let mut map = Map::new();
                map.insert("type".to_string(), Value::String("LLMJudge".to_string()));
                map.insert("model_name".to_string(), Value::String(model_name.clone()));
                map.insert(
                    "prompt_template".to_string(),
                    Value::String(prompt_template.clone()),
                );
                map.insert(
                    "approval_threshold".to_string(),
                    Value::Number(
                        serde_json::Number::from_f64(*approval_threshold)
                            .expect("Invalid f64 for approval_threshold"),
                    ),
                );
                Value::Object(map)
            }
            EvaluationType::ContextualJudge { .. } => {
                let mut map = Map::new();
                map.insert(
                    "type".to_string(),
                    Value::String("ContextualJudge".to_string()),
                );
                map.insert(
                    "context_fn".to_string(),
                    Value::String("Function pointer (not serializable)".to_string()),
                );
                Value::Object(map)
            }
        }
    }
}

impl Evaluation {
    pub fn to_json(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_string(), Value::String(self.name.clone()));
        map.insert("public".to_string(), Value::Bool(self.encrypted));
        map.insert(
            "evaluation_type".to_string(),
            self.evaluation_type.to_json(),
        );
        map
    }
}
