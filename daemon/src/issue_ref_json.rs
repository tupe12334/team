use crate::proto::{issue_ref, CentyIssueRef, GitHubIssueRef, IssueRef, JiraIssueRef};
use serde::Deserialize;

pub(crate) enum IssueRefJson {
    Github {
        organization: String,
        repository: String,
        number: String,
    },
    Centy {
        organization: String,
        repository: String,
        number: String,
    },
    Jira {
        id: String,
    },
    Link {
        url: String,
    },
}

impl std::fmt::Display for IssueRefJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueRefJson::Github { organization, repository, number } => {
                write!(f, "github:{}/{}#{}", organization, repository, number)
            }
            IssueRefJson::Centy { organization, repository, number } => {
                write!(f, "centy:{}/{}#{}", organization, repository, number)
            }
            IssueRefJson::Jira { id } => write!(f, "jira:{}", id),
            IssueRefJson::Link { url } => write!(f, "link:{}", url),
        }
    }
}

impl IssueRefJson {
    fn parse(s: &str) -> Result<Self, String> {
        if let Some(rest) = s.strip_prefix("github:") {
            let (repo_path, number) = rest.split_once('#').ok_or("missing '#' in github ref")?;
            let (organization, repository) = repo_path.split_once('/').ok_or("missing '/' in github ref")?;
            return Ok(IssueRefJson::Github {
                organization: organization.to_string(),
                repository: repository.to_string(),
                number: number.to_string(),
            });
        }
        if let Some(rest) = s.strip_prefix("centy:") {
            let (repo_path, number) = rest.split_once('#').ok_or("missing '#' in centy ref")?;
            let (organization, repository) = repo_path.split_once('/').ok_or("missing '/' in centy ref")?;
            return Ok(IssueRefJson::Centy {
                organization: organization.to_string(),
                repository: repository.to_string(),
                number: number.to_string(),
            });
        }
        if let Some(id) = s.strip_prefix("jira:") {
            return Ok(IssueRefJson::Jira { id: id.to_string() });
        }
        if let Some(url) = s.strip_prefix("link:") {
            return Ok(IssueRefJson::Link { url: url.to_string() });
        }
        Err(format!("unknown issue_ref format: {}", s))
    }
}

impl serde::Serialize for IssueRefJson {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())  // uses Display impl
    }
}

// Legacy object format for backward-compatible deserialization of old queue files.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IssueRefJsonLegacy {
    Github { organization: String, repository: String, number: String },
    Centy { organization: String, repository: String, number: String },
    Jira { id: String },
    Link { url: String },
}

impl<'de> serde::Deserialize<'de> for IssueRefJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, Visitor};

        struct IssueRefJsonVisitor;

        impl<'de> Visitor<'de> for IssueRefJsonVisitor {
            type Value = IssueRefJson;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a string like 'github:org/repo#number' or a legacy issue_ref object")
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                IssueRefJson::parse(s).map_err(de::Error::custom)
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let legacy = IssueRefJsonLegacy::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(match legacy {
                    IssueRefJsonLegacy::Github { organization, repository, number } => {
                        IssueRefJson::Github { organization, repository, number }
                    }
                    IssueRefJsonLegacy::Centy { organization, repository, number } => {
                        IssueRefJson::Centy { organization, repository, number }
                    }
                    IssueRefJsonLegacy::Jira { id } => IssueRefJson::Jira { id },
                    IssueRefJsonLegacy::Link { url } => IssueRefJson::Link { url },
                })
            }
        }

        deserializer.deserialize_any(IssueRefJsonVisitor)
    }
}

impl From<IssueRef> for IssueRefJson {
    fn from(r: IssueRef) -> Self {
        match r.r#ref {
            Some(issue_ref::Ref::Github(g)) => IssueRefJson::Github {
                organization: g.organization,
                repository: g.repository,
                number: g.number,
            },
            Some(issue_ref::Ref::Centy(c)) => IssueRefJson::Centy {
                organization: c.organization,
                repository: c.repository,
                number: c.number,
            },
            Some(issue_ref::Ref::Jira(j)) => IssueRefJson::Jira { id: j.id },
            None => unreachable!("IssueRef always has a ref variant"),
        }
    }
}

pub(crate) fn issue_ref_json_to_proto(j: IssueRefJson) -> Option<IssueRef> {
    let r = match j {
        IssueRefJson::Github { organization, repository, number } => {
            issue_ref::Ref::Github(GitHubIssueRef { organization, repository, number })
        }
        IssueRefJson::Centy { organization, repository, number } => {
            issue_ref::Ref::Centy(CentyIssueRef { organization, repository, number })
        }
        IssueRefJson::Jira { id } => issue_ref::Ref::Jira(JiraIssueRef { id }),
        // Link refs were valid in old queue files but are no longer stored.
        IssueRefJson::Link { .. } => return None,
    };
    Some(IssueRef { r#ref: Some(r) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn github_proto(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        }))}
    }
    fn centy_proto(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        }))}
    }
    fn jira_proto(id: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Jira(JiraIssueRef { id: id.into() })) }
    }

    // --- Display (serialization format) ---

    #[test]
    fn github_display() {
        let j = IssueRefJson::Github { organization: "acme".into(), repository: "backend".into(), number: "42".into() };
        assert_eq!(j.to_string(), "github:acme/backend#42");
    }

    #[test]
    fn centy_display() {
        let j = IssueRefJson::Centy { organization: "acme".into(), repository: "proj".into(), number: "7".into() };
        assert_eq!(j.to_string(), "centy:acme/proj#7");
    }

    #[test]
    fn jira_display() {
        let j = IssueRefJson::Jira { id: "PROJ-123".into() };
        assert_eq!(j.to_string(), "jira:PROJ-123");
    }

    #[test]
    fn link_display() {
        let j = IssueRefJson::Link { url: "https://app.centy.io/acme/proj/issues/42".into() };
        assert_eq!(j.to_string(), "link:https://app.centy.io/acme/proj/issues/42");
    }

    // --- Parse (deserialization) ---

    #[test]
    fn parse_github_string() {
        let j: IssueRefJson = serde_json::from_str(r#""github:acme/backend#42""#).unwrap();
        assert_eq!(j.to_string(), "github:acme/backend#42");
    }

    #[test]
    fn parse_centy_string() {
        let j: IssueRefJson = serde_json::from_str(r#""centy:acme/proj#7""#).unwrap();
        assert_eq!(j.to_string(), "centy:acme/proj#7");
    }

    #[test]
    fn parse_jira_string() {
        let j: IssueRefJson = serde_json::from_str(r#""jira:PROJ-123""#).unwrap();
        assert_eq!(j.to_string(), "jira:PROJ-123");
    }

    #[test]
    fn parse_link_string() {
        let j: IssueRefJson = serde_json::from_str(r#""link:https://example.com/foo""#).unwrap();
        assert!(matches!(j, IssueRefJson::Link { .. }));
    }

    #[test]
    fn parse_invalid_string_errors() {
        assert!(serde_json::from_str::<IssueRefJson>(r#""unknown:stuff""#).is_err());
    }

    #[test]
    fn parse_github_missing_hash_errors() {
        // No '#' separator — split_once('#') returns None → "missing '#' in github ref"
        match IssueRefJson::parse("github:acme/backend") {
            Err(e) => assert!(e.contains("missing '#'"), "got: {e}"),
            Ok(_) => panic!("expected parse error"),
        }
    }

    #[test]
    fn parse_github_missing_slash_errors() {
        // Has '#' but no '/' in the repo path — split_once('/') fails → "missing '/' in github ref"
        match IssueRefJson::parse("github:backend#42") {
            Err(e) => assert!(e.contains("missing '/'"), "got: {e}"),
            Ok(_) => panic!("expected parse error"),
        }
    }

    #[test]
    fn parse_centy_missing_hash_errors() {
        match IssueRefJson::parse("centy:acme/proj") {
            Err(e) => assert!(e.contains("missing '#'"), "got: {e}"),
            Ok(_) => panic!("expected parse error"),
        }
    }

    #[test]
    fn parse_centy_missing_slash_errors() {
        match IssueRefJson::parse("centy:proj#7") {
            Err(e) => assert!(e.contains("missing '/'"), "got: {e}"),
            Ok(_) => panic!("expected parse error"),
        }
    }

    // --- Legacy object format (backward compat) ---

    #[test]
    fn parse_legacy_github_object() {
        let json = r#"{"type":"github","organization":"acme","repository":"backend","number":"42"}"#;
        let j: IssueRefJson = serde_json::from_str(json).unwrap();
        assert_eq!(j.to_string(), "github:acme/backend#42");
    }

    #[test]
    fn parse_legacy_centy_object() {
        let json = r#"{"type":"centy","organization":"acme","repository":"proj","number":"5"}"#;
        let j: IssueRefJson = serde_json::from_str(json).unwrap();
        assert_eq!(j.to_string(), "centy:acme/proj#5");
    }

    #[test]
    fn parse_legacy_jira_object() {
        let json = r#"{"type":"jira","id":"BUG-99"}"#;
        let j: IssueRefJson = serde_json::from_str(json).unwrap();
        assert_eq!(j.to_string(), "jira:BUG-99");
    }

    // --- Proto conversion round-trips ---

    #[test]
    fn github_proto_round_trip() {
        let proto = github_proto("acme", "backend", "42");
        let j = IssueRefJson::from(proto);
        let restored = issue_ref_json_to_proto(j).unwrap();
        assert_eq!(restored, github_proto("acme", "backend", "42"));
    }

    #[test]
    fn centy_proto_round_trip() {
        let proto = centy_proto("acme", "proj", "7");
        let j = IssueRefJson::from(proto);
        let restored = issue_ref_json_to_proto(j).unwrap();
        assert_eq!(restored, centy_proto("acme", "proj", "7"));
    }

    #[test]
    fn jira_proto_round_trip() {
        let proto = jira_proto("PROJ-123");
        let j = IssueRefJson::from(proto);
        let restored = issue_ref_json_to_proto(j).unwrap();
        assert_eq!(restored, jira_proto("PROJ-123"));
    }

    #[test]
    fn link_json_to_proto_returns_none() {
        let j = IssueRefJson::Link { url: "https://example.com".into() };
        assert!(issue_ref_json_to_proto(j).is_none());
    }

    // --- JSON serde round-trips ---

    #[test]
    fn github_serde_round_trip() {
        let j = IssueRefJson::Github { organization: "acme".into(), repository: "backend".into(), number: "42".into() };
        let serialized = serde_json::to_string(&j).unwrap();
        let restored: IssueRefJson = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.to_string(), j.to_string());
    }

    #[test]
    fn centy_serde_round_trip() {
        let j = IssueRefJson::Centy { organization: "acme".into(), repository: "proj".into(), number: "7".into() };
        let serialized = serde_json::to_string(&j).unwrap();
        let restored: IssueRefJson = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.to_string(), j.to_string());
    }

    #[test]
    fn jira_serde_round_trip() {
        let j = IssueRefJson::Jira { id: "PROJ-123".into() };
        let serialized = serde_json::to_string(&j).unwrap();
        let restored: IssueRefJson = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.to_string(), j.to_string());
    }

    #[test]
    fn link_serde_round_trip() {
        let j = IssueRefJson::Link { url: "https://github.com/acme/repo/issues/1".into() };
        let serialized = serde_json::to_string(&j).unwrap();
        let restored: IssueRefJson = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.to_string(), j.to_string());
    }
}
