use crate::detect::Agent;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct AgentsConfig {
    #[serde(flatten)]
    pub agents: BTreeMap<String, AgentCommandConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct AgentCommandConfig {
    pub command: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl<'de> Deserialize<'de> for AgentCommandConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AgentCommandVisitor;

        impl<'de> Visitor<'de> for AgentCommandVisitor {
            type Value = AgentCommandConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string command or a table with command and aliases")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let trimmed = value.trim();
                Ok(AgentCommandConfig {
                    command: if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    },
                    aliases: Vec::new(),
                })
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut command = None;
                let mut aliases = Vec::new();

                while let Some(key) = access.next_key::<String>()? {
                    match key.as_str() {
                        "command" | "executable" | "bin" | "path" => {
                            command = access.next_value::<Option<String>>()?;
                        }
                        "aliases" | "alias" => {
                            aliases = access.next_value::<Vec<String>>()?;
                        }
                        _ => {
                            let _ = access.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                Ok(AgentCommandConfig {
                    command: command.and_then(|c| {
                        let trimmed = c.trim().to_string();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed)
                        }
                    }),
                    aliases,
                })
            }
        }

        deserializer.deserialize_any(AgentCommandVisitor)
    }
}

impl<'de> Deserialize<'de> for AgentsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AgentsConfigVisitor;

        impl<'de> Visitor<'de> for AgentsConfigVisitor {
            type Value = AgentsConfig;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a table of agent configurations")
            }

            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut agents = BTreeMap::new();
                while let Some(key) = access.next_key::<String>()? {
                    let value = access.next_value::<AgentCommandConfig>()?;
                    agents.insert(key, value);
                }
                Ok(AgentsConfig { agents })
            }
        }

        deserializer.deserialize_map(AgentsConfigVisitor)
    }
}

pub fn parse_agent_key(key: &str) -> Option<Agent> {
    let normalized = key.trim().to_lowercase().replace('_', "-");
    match normalized.as_str() {
        "pi" => Some(Agent::Pi),
        "claude" | "claude-code" => Some(Agent::Claude),
        "codex" => Some(Agent::Codex),
        "gemini" => Some(Agent::Gemini),
        "cursor" | "cursor-agent" => Some(Agent::Cursor),
        "devin" | "devin-cli" => Some(Agent::Devin),
        "antigravity" | "agy" | "antigravity-cli" | "agycli" => Some(Agent::Antigravity),
        "cline" => Some(Agent::Cline),
        "omp" => Some(Agent::Omp),
        "mastracode" | "mastra-code" => Some(Agent::Mastracode),
        "opencode" | "open-code" | "opencode2" => Some(Agent::OpenCode),
        "copilot" | "github-copilot" | "ghcs" => Some(Agent::GithubCopilot),
        "kimi" | "kimi-code" => Some(Agent::Kimi),
        "kiro" | "kiro-cli" => Some(Agent::Kiro),
        "droid" => Some(Agent::Droid),
        "amp" | "amp-local" => Some(Agent::Amp),
        "grok" | "grok-build" => Some(Agent::Grok),
        "hermes" | "hermes-agent" => Some(Agent::Hermes),
        "kilo" | "kilo-code" => Some(Agent::Kilo),
        "qodercli" | "qoder" => Some(Agent::Qodercli),
        "qwen" | "qwen-code" => Some(Agent::Qwen),
        "maki" => Some(Agent::Maki),
        "muse" | "muse-code" => Some(Agent::Muse),
        _ => None,
    }
}

impl AgentsConfig {
    pub fn get(&self, agent: Agent) -> Option<&AgentCommandConfig> {
        self.agents.iter().find_map(|(k, v)| {
            if parse_agent_key(k) == Some(agent) {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn agent_command(&self, agent: Agent) -> Option<&str> {
        self.get(agent).and_then(|c| c.command.as_deref())
    }

    pub fn agent_aliases(&self, agent: Agent) -> &[String] {
        self.get(agent).map(|c| c.aliases.as_slice()).unwrap_or(&[])
    }

    #[allow(dead_code)]
    pub fn antigravity_command(&self) -> Option<&str> {
        self.agent_command(Agent::Antigravity)
    }

    #[allow(dead_code)]
    pub fn antigravity_aliases(&self) -> &[String] {
        self.agent_aliases(Agent::Antigravity)
    }
}

static ACTIVE_AGENTS_CONFIG: OnceLock<RwLock<AgentsConfig>> = OnceLock::new();

pub(crate) fn set_active_agents_config(agents: AgentsConfig) {
    let lock = ACTIVE_AGENTS_CONFIG.get_or_init(|| RwLock::new(AgentsConfig::default()));
    match lock.write() {
        Ok(mut guard) => *guard = agents,
        Err(poisoned) => *poisoned.into_inner() = agents,
    }
}

pub fn active_agents_config() -> AgentsConfig {
    let lock = ACTIVE_AGENTS_CONFIG.get_or_init(|| RwLock::new(AgentsConfig::default()));
    match lock.read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

pub fn configured_agent_command(agent: Agent) -> Option<String> {
    active_agents_config()
        .agent_command(agent)
        .map(|s| s.to_string())
}

#[allow(dead_code)]
pub fn configured_agent_aliases(agent: Agent) -> Vec<String> {
    active_agents_config().agent_aliases(agent).to_vec()
}

pub fn is_configured_agent_alias(name: &str) -> Option<Agent> {
    let active = active_agents_config();
    for (key, conf) in &active.agents {
        if let Some(agent) = parse_agent_key(key) {
            if let Some(cmd) = conf.command.as_deref() {
                if crate::detect::path_basename(cmd).eq_ignore_ascii_case(name) {
                    return Some(agent);
                }
            }
            if conf
                .aliases
                .iter()
                .any(|alias| crate::detect::path_basename(alias).eq_ignore_ascii_case(name))
            {
                return Some(agent);
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn configured_antigravity_command() -> Option<String> {
    configured_agent_command(Agent::Antigravity)
}

#[allow(dead_code)]
pub fn configured_antigravity_aliases() -> Vec<String> {
    configured_agent_aliases(Agent::Antigravity)
}

#[allow(dead_code)]
pub fn is_configured_antigravity_alias(name: &str) -> bool {
    is_configured_agent_alias(name) == Some(Agent::Antigravity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agents_config_shorthand_string() {
        let toml_str = r#"
            antigravity = "my-custom-agy"
            claude = "custom-claude-bin"
        "#;
        let config: AgentsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.agent_command(Agent::Antigravity),
            Some("my-custom-agy")
        );
        assert_eq!(
            config.agent_command(Agent::Claude),
            Some("custom-claude-bin")
        );
        assert!(config.agent_aliases(Agent::Antigravity).is_empty());
    }

    #[test]
    fn parse_agents_config_agy_alias_shorthand() {
        let toml_str = r#"
            agy = "antigravity-bin"
        "#;
        let config: AgentsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.agent_command(Agent::Antigravity),
            Some("antigravity-bin")
        );
        assert!(config.agent_aliases(Agent::Antigravity).is_empty());
    }

    #[test]
    fn parse_agents_config_table_with_aliases() {
        let toml_str = r#"
            [antigravity]
            command = "/opt/bin/custom-antigravity"
            aliases = ["agy-dev", "custom-agy"]

            [gemini]
            command = "/opt/bin/gemini-pro"
            aliases = ["gemini-beta"]
        "#;
        let config: AgentsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.agent_command(Agent::Antigravity),
            Some("/opt/bin/custom-antigravity")
        );
        assert_eq!(
            config.agent_aliases(Agent::Antigravity),
            &["agy-dev".to_string(), "custom-agy".to_string()]
        );
        assert_eq!(
            config.agent_command(Agent::Gemini),
            Some("/opt/bin/gemini-pro")
        );
        assert_eq!(
            config.agent_aliases(Agent::Gemini),
            &["gemini-beta".to_string()]
        );
    }
}
