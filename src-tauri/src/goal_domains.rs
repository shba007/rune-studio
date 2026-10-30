//! Compiled goal taxonomy shared with Studio; labels may evolve, stored IDs stay stable.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::LazyLock;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    pub id: String,
    pub name: String,
    pub zone: String,
    pub sub_domains: Vec<SubDomain>,
}

#[derive(Deserialize)]
pub struct SubDomain {
    pub id: String,
    pub name: String,
}

pub static REGISTRY: LazyLock<Vec<Domain>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("goal_domains.json")).expect("valid compiled goal registry")
});

pub fn find(answer: &str) -> Option<&'static Domain> {
    let answer = answer.trim();
    REGISTRY.iter().find(|domain| {
        domain.id.eq_ignore_ascii_case(answer) || domain.name.eq_ignore_ascii_case(answer)
    })
}

pub fn context() -> Value {
    Value::Array(
        REGISTRY
            .iter()
            .map(|domain| {
                serde_json::json!({
                    "id": domain.id, "name": domain.name, "zone": domain.zone,
                })
            })
            .collect(),
    )
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubDomainSelection {
    pub id: String,
    #[serde(default)]
    pub answers: Vec<Value>,
    #[serde(default)]
    pub skipped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Manual,
    Auto,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Auto => "auto",
        }
    }
}

pub struct Selection {
    pub mode: Mode,
    pub domain: Option<String>,
    pub sub_domains: Vec<SubDomainSelection>,
}

impl Selection {
    pub fn create(
        text: Option<String>,
        domain: Option<String>,
        mode: Option<&str>,
        sub_domains: Vec<String>,
    ) -> Result<(String, Self), String> {
        let mut selection = Self::manual(None, domain, sub_domains)?;
        match mode.unwrap_or("manual") {
            "manual" => {
                let text = text.ok_or("Enter a goal for Manual creation")?;
                if text.trim().is_empty() || text.chars().count() > 4000 {
                    return Err("Enter a goal of 1 to 4000 characters".into());
                }
                Ok((text, selection))
            }
            "auto" => {
                if text.is_some() {
                    return Err("Auto starts from a domain and answers, without goal text".into());
                }
                if selection.domain.is_none() {
                    return Err("Choose a domain explicitly before starting Auto".into());
                }
                selection.mode = Mode::Auto;
                Ok((String::new(), selection))
            }
            _ => Err("Choose Manual or Auto goal creation".into()),
        }
    }

    pub fn manual(
        mode: Option<&str>,
        domain: Option<String>,
        sub_domains: Vec<String>,
    ) -> Result<Self, String> {
        if mode.unwrap_or("manual") != "manual" {
            return Err("Only Manual goal creation is supported by this version".into());
        }
        let definition = domain
            .as_deref()
            .map(|id| {
                REGISTRY
                    .iter()
                    .find(|item| item.id == id)
                    .ok_or("Choose a supported goal domain")
            })
            .transpose()?;
        let mut selected = Vec::new();
        for id in sub_domains {
            if !definition.is_some_and(|d| d.sub_domains.iter().any(|s| s.id == id)) {
                return Err("Choose sub-domains belonging to the selected domain".into());
            }
            if selected.iter().any(|s: &SubDomainSelection| s.id == id) {
                return Err("Choose each sub-domain only once".into());
            }
            selected.push(SubDomainSelection {
                id,
                answers: vec![],
                skipped: false,
            });
        }
        Ok(Self {
            mode: Mode::Manual,
            domain,
            sub_domains: selected,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_creation_requires_an_explicit_domain_and_no_goal_text() {
        assert!(Selection::create(None, None, Some("auto"), vec![]).is_err());
        assert!(
            Selection::create(
                Some(String::new()),
                Some("physical".into()),
                Some("auto"),
                vec![]
            )
            .is_err()
        );
        assert!(
            Selection::create(
                Some("A typed goal".into()),
                Some("physical".into()),
                Some("auto"),
                vec![]
            )
            .is_err()
        );
        assert!(Selection::create(None, Some("invalid".into()), Some("auto"), vec![]).is_err());
        assert!(
            Selection::create(
                None,
                Some("physical".into()),
                Some("auto"),
                vec!["digital-hygiene".into()]
            )
            .is_err()
        );
        let (text, selection) =
            Selection::create(None, Some("cognitive".into()), Some("auto"), vec![]).unwrap();
        assert!(text.is_empty());
        assert!(selection.mode == Mode::Auto);
        assert_eq!(selection.domain.as_deref(), Some("cognitive"));
        let (text, selection) =
            Selection::create(Some("A legacy text-only goal".into()), None, None, vec![]).unwrap();
        assert_eq!(text, "A legacy text-only goal");
        assert!(selection.mode == Mode::Manual);
        assert!(Selection::create(None, None, None, vec![]).is_err());
        assert!(Selection::create(Some(" ".into()), None, None, vec![]).is_err());
        assert!(Selection::create(Some("x".repeat(4001)), None, None, vec![]).is_err());
        assert!(Selection::create(None, None, Some("invalid"), vec![]).is_err());
    }

    #[test]
    fn manual_accepts_legacy_calls_and_validates_domain_membership() {
        assert!(
            Selection::manual(None, None, vec![])
                .unwrap()
                .domain
                .is_none()
        );
        assert!(Selection::manual(Some("auto"), Some("physical".into()), vec![]).is_err());
        assert!(Selection::manual(Some("invalid"), None, vec![]).is_err());
        assert!(Selection::manual(None, Some("invalid".into()), vec![]).is_err());
        assert!(Selection::manual(None, None, vec!["digital-hygiene".into()]).is_err());
        assert!(
            Selection::manual(
                None,
                Some("physical".into()),
                vec!["digital-hygiene".into()]
            )
            .is_err()
        );
        assert!(
            Selection::manual(
                None,
                Some("environment".into()),
                vec!["digital-hygiene".into(); 2]
            )
            .is_err()
        );
        let selection = Selection::manual(
            None,
            Some("environment".into()),
            vec!["geographic-fit".into()],
        )
        .unwrap();
        assert_eq!(selection.sub_domains[0].id, "geographic-fit");
    }

    #[test]
    fn shared_registry_has_unique_stable_ids_and_approved_taxonomy() {
        let data: Value = serde_json::from_str(include_str!("goal_domains.json")).unwrap();
        let domains = data.as_array().unwrap();
        assert_eq!(domains.len(), 12);
        let mut ids = std::collections::HashSet::new();
        let mut numbers = std::collections::HashSet::new();
        let mut count = 0;
        for d in domains {
            assert!(ids.insert(d["id"].as_str().unwrap()));
            assert!(numbers.insert(d["number"].as_u64().unwrap()));
            let subs = d["subDomains"].as_array().unwrap();
            let expected = if matches!(d["id"].as_str(), Some("family" | "legacy")) {
                5
            } else {
                6
            };
            assert_eq!(subs.len(), expected);
            let mut sub_ids = std::collections::HashSet::new();
            for s in subs {
                assert!(sub_ids.insert(s["id"].as_str().unwrap()));
            }
            count += subs.len();
        }
        assert_eq!(count, 70);
        assert_eq!(domains.iter().filter(|d| d["zone"] == "engine").count(), 5);
        assert_eq!(
            domains.iter().find(|d| d["id"] == "cognitive").unwrap()["zone"],
            "sanctuary"
        );
        let environment = domains.iter().find(|d| d["id"] == "environment").unwrap();
        assert_eq!(environment["subDomains"][5]["id"], "geographic-fit");
        assert_eq!(environment["subDomains"][5]["name"], "Sensory Fit");
    }
}
