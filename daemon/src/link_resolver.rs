use crate::centy_resolver;
use crate::proto::{issue_ref, CentyIssueRef, GitHubIssueRef, IssueRef, JiraIssueRef};

/// Resolve a raw link URL to a concrete IssueRef.
/// Returns an error string if the URL doesn't match any known provider pattern.
pub async fn resolve(url: &str) -> Result<IssueRef, String> {
    if let Some(r) = try_github(url).or_else(|| try_jira(url)) {
        return Ok(r);
    }
    try_centy(url).await?.ok_or_else(|| {
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
    if number.parse::<u32>().is_err() {
        return None;
    }
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

/// Returns `Ok(None)` if the URL doesn't match the Centy pattern.
/// Returns `Ok(Some(_))` on success.
/// Returns `Err(_)` if the URL matches but UUID-to-display-number resolution fails.
///
/// Centy URLs embed the item UUID rather than the display number, e.g.
/// `https://app.centy.io/acme/proj/issues/6f4853a9-3d82-4013-b909-c2d637f44541`.
/// When a UUID is detected the centy CLI is queried to obtain the integer
/// display number that `worktree open centy:<n>` requires.
async fn try_centy(url: &str) -> Result<Option<IssueRef>, String> {
    let path = match url
        .strip_prefix("https://app.centy.io/")
        .or_else(|| url.strip_prefix("http://app.centy.io/"))
    {
        Some(p) => p,
        None => return Ok(None),
    };
    let mut parts = path.splitn(5, '/');
    let Some(org) = parts.next().filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let Some(project) = parts.next().filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let Some(kind) = parts.next() else {
        return Ok(None);
    };
    if kind != "issues" {
        return Ok(None);
    }
    let Some(number_raw) = parts.next().filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let id = number_raw.split('?').next().unwrap_or(number_raw);
    let id = id.split('#').next().unwrap_or(id);

    let number = if centy_resolver::is_uuid(id) {
        centy_resolver::resolve_centy_uuid(id).await?
    } else {
        id.to_string()
    };

    Ok(Some(IssueRef {
        r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
            organization: org.to_string(),
            repository: project.to_string(),
            number,
        })),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::issue_ref;

    fn github(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef {
            r#ref: Some(issue_ref::Ref::Github(GitHubIssueRef {
                organization: org.into(),
                repository: repo.into(),
                number: number.into(),
            })),
        }
    }
    fn jira(id: &str) -> IssueRef {
        IssueRef { r#ref: Some(issue_ref::Ref::Jira(JiraIssueRef { id: id.into() })) }
    }
    fn centy(org: &str, repo: &str, number: &str) -> IssueRef {
        IssueRef {
            r#ref: Some(issue_ref::Ref::Centy(CentyIssueRef {
                organization: org.into(),
                repository: repo.into(),
                number: number.into(),
            })),
        }
    }

    #[tokio::test]
    async fn github_issues_url() {
        assert_eq!(
            resolve("https://github.com/acme/my-repo/issues/42").await.unwrap(),
            github("acme", "my-repo", "42")
        );
    }
    #[tokio::test]
    async fn github_pull_url() {
        assert_eq!(
            resolve("https://github.com/acme/my-repo/pull/7").await.unwrap(),
            github("acme", "my-repo", "7")
        );
    }
    #[tokio::test]
    async fn github_url_strips_query_and_fragment() {
        assert_eq!(
            resolve("https://github.com/acme/my-repo/issues/99?ref=foo#bar").await.unwrap(),
            github("acme", "my-repo", "99")
        );
    }
    #[tokio::test]
    async fn github_http_scheme() {
        assert_eq!(
            resolve("http://github.com/acme/my-repo/issues/1").await.unwrap(),
            github("acme", "my-repo", "1")
        );
    }
    #[tokio::test]
    async fn github_missing_number_returns_err() {
        assert!(resolve("https://github.com/acme/my-repo/issues/").await.is_err());
    }
    #[tokio::test]
    async fn github_wrong_kind_returns_err() {
        assert!(resolve("https://github.com/acme/my-repo/blob/main/README.md").await.is_err());
    }

    #[tokio::test]
    async fn jira_atlassian_url() {
        assert_eq!(
            resolve("https://acme.atlassian.net/browse/PROJ-123").await.unwrap(),
            jira("PROJ-123")
        );
    }
    #[tokio::test]
    async fn jira_jira_com_url() {
        assert_eq!(
            resolve("https://acme.jira.com/browse/BUG-7").await.unwrap(),
            jira("BUG-7")
        );
    }
    #[tokio::test]
    async fn jira_url_strips_query() {
        assert_eq!(
            resolve("https://acme.atlassian.net/browse/PROJ-99?selected=tab").await.unwrap(),
            jira("PROJ-99")
        );
    }

    #[tokio::test]
    async fn centy_url() {
        assert_eq!(
            resolve("https://app.centy.io/acme/my-project/issues/5").await.unwrap(),
            centy("acme", "my-project", "5")
        );
    }
    #[tokio::test]
    async fn centy_http_scheme() {
        assert_eq!(
            resolve("http://app.centy.io/acme/proj/issues/3").await.unwrap(),
            centy("acme", "proj", "3")
        );
    }
    #[tokio::test]
    async fn centy_url_strips_query() {
        assert_eq!(
            resolve("https://app.centy.io/acme/proj/issues/10?foo=bar").await.unwrap(),
            centy("acme", "proj", "10")
        );
    }

    /// UUID URLs must trigger centy CLI resolution, not fall through to "unknown provider".
    #[tokio::test]
    async fn centy_uuid_url_attempts_resolution() {
        let result =
            resolve("https://app.centy.io/acme/proj/issues/6f4853a9-3d82-4013-b909-c2d637f44541")
                .await;
        match result {
            Ok(_) => {} // centy resolved it (integration environment)
            Err(e) => {
                assert!(
                    !e.starts_with("cannot resolve link"),
                    "expected a centy resolution error, got: {e}"
                );
            }
        }
    }

    #[tokio::test]
    async fn github_non_integer_number_returns_err() {
        assert!(resolve("https://github.com/acme/my-repo/issues/not-a-number").await.is_err());
    }
    #[tokio::test]
    async fn github_float_number_returns_err() {
        assert!(resolve("https://github.com/acme/my-repo/issues/1.5").await.is_err());
    }

    #[tokio::test]
    async fn unknown_url_returns_err() {
        assert!(resolve("https://gitlab.com/acme/repo/-/issues/1").await.is_err());
    }
    #[tokio::test]
    async fn empty_url_returns_err() {
        assert!(resolve("").await.is_err());
    }
}
