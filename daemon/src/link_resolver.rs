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
    // https://centy.io/{org}/{project}/issues/{number}
    let path = url
        .strip_prefix("https://centy.io/")
        .or_else(|| url.strip_prefix("http://centy.io/"))?;
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
