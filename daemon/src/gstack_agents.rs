pub struct AgentDef {
    pub name: &'static str,
    pub description: &'static str,
}

static AGENTS: &[AgentDef] = &[
    AgentDef { name: "autoplan", description: "Auto-review pipeline — runs CEO, design, and eng review skills sequentially with auto-decisions." },
    AgentDef { name: "benchmark", description: "Performance regression detection using the browse daemon. Establishes baselines for page load times and Core Web Vitals." },
    AgentDef { name: "browse", description: "Fast headless browser for QA testing and site dogfooding. Navigate any URL, interact with elements, verify page state." },
    AgentDef { name: "canary", description: "Post-deploy canary monitoring. Watches the live app for console errors, performance regressions, and page failures." },
    AgentDef { name: "careful", description: "Safety guardrails for destructive commands. Warns before rm -rf, DROP TABLE, force-push, git reset --hard, and similar operations." },
    AgentDef { name: "checkpoint", description: "Save and resume working state checkpoints. Captures git state, decisions made, and remaining work." },
    AgentDef { name: "codex", description: "OpenAI Codex CLI wrapper — code review, adversarial challenge mode, and multi-model comparison." },
    AgentDef { name: "connect-chrome", description: "Launch real Chrome controlled by gstack with the Side Panel extension auto-loaded." },
    AgentDef { name: "cso", description: "Chief Security Officer mode. Infrastructure-first security audit: secrets, dependencies, CI/CD pipeline, LLM/AI security." },
    AgentDef { name: "design-consultation", description: "Design consultation: understands your product, researches the landscape, proposes a complete design system." },
    AgentDef { name: "design-html", description: "Design finalization: generates production-quality HTML/CSS from approved mockups." },
    AgentDef { name: "design-review", description: "Designer's eye QA: finds visual inconsistency, spacing issues, hierarchy problems, AI slop patterns — then fixes them." },
    AgentDef { name: "design-shotgun", description: "Design shotgun: generate multiple AI design variants, open a comparison board, collect structured feedback, and iterate." },
    AgentDef { name: "devex-review", description: "Live developer experience audit. Uses the browse tool to actually test the developer experience." },
    AgentDef { name: "document-release", description: "Post-ship documentation update. Reads all project docs, cross-references the diff, updates README/ARCHITECTURE/CONTRIBUTING/CLAUDE.md." },
    AgentDef { name: "freeze", description: "Restrict file edits to a specific directory for the session. Blocks Edit and Write outside the allowed path." },
    AgentDef { name: "gstack-upgrade", description: "Upgrade gstack to the latest version. Detects global vs vendored install, runs the upgrade, and shows what's new." },
    AgentDef { name: "guard", description: "Full safety mode: destructive command warnings + directory-scoped edits. Combines careful and freeze." },
    AgentDef { name: "health", description: "Code quality dashboard. Wraps existing project tools (type checker, linter, test runner, dead code detector) and computes a weighted composite score." },
    AgentDef { name: "investigate", description: "Systematic debugging with root cause investigation. Four phases: investigate, analyze, hypothesize, implement." },
    AgentDef { name: "land-and-deploy", description: "Land and deploy workflow. Merges the PR, waits for CI and deploy, verifies production health via canary checks." },
    AgentDef { name: "learn", description: "Manage project learnings. Review, search, prune, and export what gstack has learned across sessions." },
    AgentDef { name: "office-hours", description: "YC Office Hours — startup mode: six forcing questions that expose demand reality, status quo, and narrowest wedge." },
    AgentDef { name: "plan-ceo-review", description: "CEO/founder-mode plan review. Rethink the problem, find the 10-star product, challenge premises, expand scope." },
    AgentDef { name: "plan-design-review", description: "Designer's eye plan review. Rates each design dimension 0-10, explains what would make it a 10." },
    AgentDef { name: "plan-devex-review", description: "Developer Experience plan review. Evaluates plans through Addy Osmani's DX framework: zero friction, learn by doing." },
    AgentDef { name: "plan-eng-review", description: "Eng manager-mode plan review. Lock in architecture, data flow, edge cases, test coverage, and performance." },
    AgentDef { name: "qa-only", description: "Report-only QA testing. Systematically tests a web application and produces a structured report — no code changes." },
    AgentDef { name: "qa", description: "Systematically QA test a web application and fix bugs found. Runs QA testing, then iteratively fixes and commits bugs." },
    AgentDef { name: "retro", description: "Weekly engineering retrospective. Analyzes commit history, work patterns, and code quality metrics." },
    AgentDef { name: "review", description: "Pre-landing PR review. Analyzes diff for SQL safety, LLM trust boundary violations, conditional side effects, and structural issues." },
    AgentDef { name: "setup-browser-cookies", description: "Import cookies from your real Chromium browser into the headless browse session." },
    AgentDef { name: "setup-deploy", description: "Configure deployment settings for land-and-deploy. Detects your deploy platform (Fly.io, Render, Vercel, Netlify, Heroku)." },
    AgentDef { name: "ship", description: "Ship workflow: detect + merge base branch, run tests, review diff, bump VERSION, update CHANGELOG, commit, push, create PR." },
    AgentDef { name: "unfreeze", description: "Clear the freeze boundary set by freeze, allowing edits to all directories again." },
];

#[allow(dead_code)]
pub fn all() -> &'static [AgentDef] {
    AGENTS
}

/// Returns all agents if `enabled` is empty, otherwise filters to only those listed.
pub fn filter(enabled: &[String]) -> Vec<&'static AgentDef> {
    if enabled.is_empty() {
        AGENTS.iter().collect()
    } else {
        AGENTS
            .iter()
            .filter(|a| enabled.iter().any(|e| e == a.name))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_returns_35_agents() {
        assert_eq!(AGENTS.len(), 35);
    }

    #[test]
    fn filter_empty_returns_all() {
        let result = filter(&[]);
        assert_eq!(result.len(), 35);
    }

    #[test]
    fn filter_subset_returns_only_listed() {
        let enabled = vec!["review".to_string(), "qa".to_string(), "ship".to_string()];
        let result = filter(&enabled);
        assert_eq!(result.len(), 3);
        let names: Vec<&str> = result.iter().map(|a| a.name).collect();
        assert!(names.contains(&"review"));
        assert!(names.contains(&"qa"));
        assert!(names.contains(&"ship"));
    }

    #[test]
    fn filter_unknown_names_are_ignored() {
        let enabled = vec!["review".to_string(), "nonexistent-agent".to_string()];
        let result = filter(&enabled);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "review");
    }

    #[test]
    fn filter_single_agent() {
        let enabled = vec!["ship".to_string()];
        let result = filter(&enabled);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "ship");
        assert!(!result[0].description.is_empty());
    }

    #[test]
    fn all_agent_names_are_nonempty_and_unique() {
        let names: Vec<&str> = AGENTS.iter().map(|a| a.name).collect();
        assert!(names.iter().all(|n| !n.is_empty()));
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "duplicate agent names detected");
    }

    #[test]
    fn all_agent_descriptions_are_nonempty() {
        assert!(AGENTS.iter().all(|a| !a.description.is_empty()));
    }
}
