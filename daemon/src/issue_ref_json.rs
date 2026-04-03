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

impl IssueRefJson {
    fn to_string(&self) -> String {
        match self {
            IssueRefJson::Github { organization, repository, number } => {
                format!("github:{}/{}#{}", organization, repository, number)
            }
            IssueRefJson::Centy { organization, repository, number } => {
                format!("centy:{}/{}#{}", organization, repository, number)
            }
            IssueRefJson::Jira { id } => format!("jira:{}", id),
            IssueRefJson::Link { url } => format!("link:{}", url),
        }
    }

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
        serializer.serialize_str(&self.to_string())
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
