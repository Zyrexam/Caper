use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HarnessId {
    Angela,
    OpenCode,
    ClaudeCode,
    Codex,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTarget {
    pub harness: HarnessId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl SessionTarget {
    pub fn from_legacy_model(model: Option<&str>) -> Self {
        let (provider, model) = model
            .and_then(|value| value.split_once(':'))
            .map(|(provider, model)| (non_empty(provider), non_empty(model)))
            .unwrap_or((None, None));
        Self {
            harness: HarnessId::Angela,
            provider,
            model,
        }
    }

    pub fn legacy_model(&self) -> Option<String> {
        if self.harness != HarnessId::Angela {
            return None;
        }
        Some(format!(
            "{}:{}",
            self.provider.as_deref()?,
            self.model.as_deref()?
        ))
    }
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub fn catalog() -> Value {
    let models = crate::api::models();
    let entries = models
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut providers = std::collections::BTreeMap::<String, Vec<String>>::new();
    for entry in entries.iter().filter_map(Value::as_str) {
        if let Some((provider, model)) = entry.split_once(':') {
            let Some(provider) = non_empty(provider) else {
                continue;
            };
            let Some(model) = non_empty(model) else {
                continue;
            };
            let items = providers.entry(provider).or_default();
            if !items.contains(&model) {
                items.push(model);
            }
        }
    }
    let provider_list = providers
        .into_iter()
        .map(|(id, models)| json!({"id": id, "models": models}))
        .collect::<Vec<_>>();
    let default_model = models.get("default_model").and_then(Value::as_str);
    let default_target = SessionTarget::from_legacy_model(default_model);
    json!({
        "defaultTarget": default_target,
        "harnesses": [
            {"id":"angela", "label":"Angela", "ready":true, "reason":Value::Null, "providers":provider_list},
            {"id":"opencode", "label":"OpenCode", "ready":false, "reason":"Harness adapter is not integrated yet", "providers":[]},
            {"id":"claude-code", "label":"Claude Code", "ready":false, "reason":"Harness adapter is not integrated yet", "providers":[]},
            {"id":"codex", "label":"Codex", "ready":false, "reason":"Harness adapter is not integrated yet", "providers":[]}
        ]
    })
}

pub fn parse_target(value: &Value, catalog: &Value) -> Result<SessionTarget, String> {
    let target: SessionTarget = serde_json::from_value(value.clone())
        .map_err(|_| "target must include a supported harness".to_string())?;
    let harness_id = serde_json::to_value(target.harness).unwrap_or(Value::Null);
    let harness = catalog
        .get("harnesses")
        .and_then(Value::as_array)
        .and_then(|all| all.iter().find(|item| item.get("id") == Some(&harness_id)))
        .ok_or_else(|| "unknown harness".to_string())?;
    if harness.get("ready") != Some(&Value::Bool(true)) {
        return Err(harness
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("harness is unavailable")
            .to_string());
    }
    let (Some(provider), Some(model)) = (target.provider.as_deref(), target.model.as_deref())
    else {
        if target.provider.is_none()
            && target.model.is_none()
            && target.harness == HarnessId::Angela
        {
            return Ok(target);
        }
        return Err("target provider and model must be selected together".into());
    };
    let known = harness
        .get("providers")
        .and_then(Value::as_array)
        .is_some_and(|providers| {
            providers.iter().any(|item| {
                item.get("id").and_then(Value::as_str) == Some(provider)
                    && item
                        .get("models")
                        .and_then(Value::as_array)
                        .is_some_and(|models| models.iter().any(|m| m.as_str() == Some(model)))
            })
        });
    if !known {
        return Err("provider or model is not available for this harness".into());
    }
    Ok(target)
}

pub fn target_config(config: Option<&Value>, target: &SessionTarget) -> Value {
    let mut config = config
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    config.insert(
        "target".into(),
        serde_json::to_value(target).unwrap_or(Value::Null),
    );
    Value::Object(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_fixture() -> Value {
        json!({"harnesses":[
            {"id":"angela", "ready":true, "providers":[{"id":"anthropic", "models":["sonnet"]}]},
            {"id":"codex", "ready":false, "reason":"adapter missing", "providers":[]}
        ]})
    }

    #[test]
    fn target_is_typed_and_validated_against_the_catalog() {
        let target = parse_target(
            &json!({"harness":"angela", "provider":"anthropic", "model":"sonnet"}),
            &catalog_fixture(),
        )
        .unwrap();
        assert_eq!(target.legacy_model().as_deref(), Some("anthropic:sonnet"));
        assert!(parse_target(
            &json!({"harness":"codex", "provider":"", "model":""}),
            &catalog_fixture()
        )
        .is_err());
        assert!(parse_target(
            &json!({"harness":"angela", "provider":"anthropic", "model":"unknown"}),
            &catalog_fixture()
        )
        .is_err());
    }

    #[test]
    fn legacy_models_remain_angela_provider_models() {
        assert_eq!(
            SessionTarget::from_legacy_model(Some("opencode:claude-sonnet")).harness,
            HarnessId::Angela
        );
    }

    #[test]
    fn writing_a_target_keeps_other_session_config() {
        let target = SessionTarget::from_legacy_model(Some("anthropic:sonnet"));
        let config = target_config(Some(&json!({"reasoningEffort":"high"})), &target);
        assert_eq!(config["reasoningEffort"], "high");
        assert_eq!(config["target"]["model"], "sonnet");
    }
}
