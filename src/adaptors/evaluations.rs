use serde_json::Value;

use crate::nibble::Nibble;

use std::{error::Error, sync::Arc};

#[derive(Debug)]
pub struct Evaluation {
    pub name: String,
    pub public: bool,
    pub evaluation_type: EvaluationType,
}

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
    public: bool,
) -> Result<(), Box<dyn Error>> {
    nibble.evaluations.push(Evaluation {
        name: name.to_string(),
        public,
        evaluation_type,
    });
    Ok(())
}
