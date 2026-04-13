use crate::proto::{issue_ref, CentyIssueRef, GitHubIssueRef, IssueRef, JiraIssueRef};

/// Resolve a raw link URL to a concrete IssueRef.
/// Returns an error string if the URL doesn't match any known provider pattern.
pub fn resolve(url: &str) -> Result<IssueRef, String> {
    try_github(url)
        .or_else(|| try_jira(url))
        .or_else(|| try_centy(url))
        .ok_or_else(|| {
            format!("cannot resolve link to a known issue provider (github/jira/centy): {url}")
        })
}

fn try_github(url: &str) -> Option<IssueRef> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    // Expect: {org}/{repo}/issues/{number} or {org}/{repo}/pull/{number}
    let mut parts = path.splitn(5, '/');
    let org = parts.next().filter(|s| !s.is_empty())?;
    let repo = parts.next().filter(|s| !s.is_empty())?;
    let kind = parts.next()?;
    if kind != "issues" && kind != "pull" {
        return None;
    }
    let number_raw = parts.next().filter(|s| !s.is_empty())?;
    let number = number_raw.split('?').next().unwrap_or(number_raw);
    let number = number.split('#').next().unwrap_or(number);
    Some(IssueRef {
        r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
            organization: org.to_string(),
            repository: repo.to_string(),
            number: number.to_string(),
        })),
    })
}

fn try_jira(url: &str) -> Option<IssueRef> {
    // https://{subdomain}.atlassian.net/browse/{ISSUE-KEY}
    // https://{company}.jira.com/browse/{ISSUE-KEY}
    if !url.contains(".atlassian.net/") && !url.contains(".jira.com/") {
        return None;
    }
    let id_raw = url.split("/browse/").nth(1)?;
    let id = id_raw.split('?').next().unwrap_or(id_raw);
    let id = id.split('#').next().unwrap_or(id);
    if id.is_empty() {
        return None;
    }
    Some(IssueRef {
        r#ref: Some(issue_ref::Ref::Jira(JiraIssueRef { id: id.to_string() })),
    })
}

fn try_centy(url: &str) -> Option<IssueRef> {
    // https://app.centy.io/{org}/{project}/issues/{id}
    let path = url
        .strip_prefix("https://app.centy.io/")
        .or_else(|| url.strip_prefix("http://app.centy.io/"))?;
    let mut parts = path.splitn(5, '/');
    let org = parts.next().filter(|s| !s.is_empty())?;
    let project = parts.next().filter(|s| !s.is_empty())?;
    let kind = parts.next()?;
    if kind != "issues" {
        return None;
    }
    let number_raw = parts.next().filter(|s| !s.is_empty())?;
    let number = number_raw.split('?').next().unwrap_or(number_raw);
    let number = number.split('#').next().unwrap_or(number);
    Some(IssueRef {
        r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
            organization: org.to_string(),
            repository: project.to_string(),
            number: number.to_string(),
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::issue_ref;

    fn github(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        }))}
    }
    fn jira(id: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Jira(JiraIssueRef { id: id.into() })) }
    }
    fn centy(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
            organization: org.into(), repository: repo.into(), number: number.into(),
        }))}
    }

    #[test]
    fn github_issues_url() {
        assert_eq!(resolve("https://github.com/acme/my-repo/issues/42").unwrap(), github("acme", "my-repo", "42"));
    }
    #[test]
    fn github_pull_url() {
        assert_eq!(resolve("https://github.com/acme/my-repo/pull/7").unwrap(), github("acme", "my-repo", "7"));
    }
    #[test]
    fn github_url_strips_query_and_fragment() {
        assert_eq!(resolve("https://github.com/acme/my-repo/issues/99?ref=foo#bar").unwrap(), github("acme", "my-repo", "99"));
    }
    #[test]
    fn github_http_scheme() {
        assert_eq!(resolve("http://github.com/acme/my-repo/issues/1").unwrap(), github("acme", "my-repo", "1"));
    }
    #[test]
    fn github_missing_number_returns_err() {
        assert!(resolve("https://github.com/acme/my-repo/issues/").is_err());
    }
    #[test]
    fn github_wrong_kind_returns_err() {
        assert!(resolve("https://github.com/acme/my-repo/blob/main/README.md").is_err());
    }

    #[test]
    fn jira_atlassian_url() {
        assert_eq!(resolve("https://acme.atlassian.net/browse/PROJ-123").unwrap(), jira("PROJ-123"));
    }
    #[test]
    fn jira_jira_com_url() {
        assert_eq!(resolve("https://acme.jira.com/browse/BUG-7").unwrap(), jira("BUG-7"));
    }
    #[test]
    fn jira_url_strips_query() {
        assert_eq!(resolve("https://acme.atlassian.net/browse/PROJ-99?selected=tab").unwrap(), jira("PROJ-99"));
    }

    #[test]
    fn centy_url() {
        assert_eq!(resolve("https://app.centy.io/acme/my-project/issues/5").unwrap(), centy("acme", "my-project", "5"));
    }
    #[test]
    fn centy_http_scheme() {
        assert_eq!(resolve("http://app.centy.io/acme/proj/issues/3").unwrap(), centy("acme", "proj", "3"));
    }
    #[test]
    fn centy_url_strips_query() {
        assert_eq!(resolve("https://app.centy.io/acme/proj/issues/10?foo=bar").unwrap(), centy("acme", "proj", "10"));
    }

    #[test]
    fn unknown_url_returns_err() {
        assert!(resolve("https://gitlab.com/acme/repo/-/issues/1").is_err());
    }
    #[test]
    fn empty_url_returns_err() {
        assert!(resolve("").is_err());
    }
}
