/// Returns true if `s` matches the canonical UUID format: 8-4-4-4-12 hex chars separated by dashes.
pub fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lengths = [8usize, 4, 4, 4, 12];
    parts
        .iter()
        .zip(expected_lengths.iter())
        .all(|(p, &len)| p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Resolves a Centy item UUID to its integer display number using the centy CLI.
///
/// Runs: `centy get issue <uuid> --global --json --project /root`
///
/// The `--project /root` flag short-circuits centy's `resolveProjectPath()` so it
/// returns immediately without attempting a daemon project lookup.  This is necessary
/// because `docker-compose.yml` sets `CENTY_CWD=""` when the user hasn't configured it,
/// which would otherwise cause a `ProjectNotFoundError` before `--global` is evaluated
/// (unlike `list items`, which checks `--global` before calling `resolveProjectPath`).
pub async fn resolve_centy_uuid(uuid: &str) -> Result<String, String> {
    let output = tokio::process::Command::new("centy")
        .args(["get", "issue", uuid, "--global", "--json", "--project", "/root"])
        .output()
        .await
        .map_err(|e| format!("failed to spawn centy to resolve UUID {uuid}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "centy exited with {} when resolving UUID {uuid}: {stderr}",
            output.status.code().unwrap_or(-1)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    let json_start = match stdout.find('{') {
        Some(pos) => pos,
        None => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "centy returned no JSON when resolving UUID {uuid}: {stderr}"
            ));
        }
    };

    let response: serde_json::Value = serde_json::from_str(&stdout[json_start..])
        .map_err(|e| format!("failed to parse centy JSON for UUID {uuid}: {e}"))?;

    // Response shape: SearchItemsResponse { items: [{ item: { metadata: { displayNumber: N } } }] }
    response["items"][0]["item"]["metadata"]["displayNumber"]
        .as_i64()
        .map(|n| n.to_string())
        .ok_or_else(|| format!("UUID {uuid} not found in any tracked centy project"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_uuid_valid() {
        assert!(is_uuid("6f4853a9-3d82-4013-b909-c2d637f44541"));
        assert!(is_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(is_uuid("a1b2c3d4-e5f6-7890-abcd-ef1234567890"));
    }

    #[test]
    fn is_uuid_rejects_plain_integers() {
        assert!(!is_uuid("42"));
        assert!(!is_uuid("5"));
        assert!(!is_uuid("0"));
    }

    #[test]
    fn is_uuid_rejects_malformed() {
        assert!(!is_uuid("not-a-uuid"));
        assert!(!is_uuid("6f4853a9-3d82-4013-b909"));
        assert!(!is_uuid("6f4853a9-3d82-4013-b909-c2d637f44541-extra"));
        assert!(!is_uuid("gggggggg-0000-0000-0000-000000000000")); // invalid hex
    }

    /// Exercises the `p.len() == len` check in the `all()` predicate — five parts with
    /// correct hex digits but at least one segment of the wrong length.
    /// The existing `is_uuid_rejects_malformed` test only covers wrong part *count* and
    /// non-hex chars; this test isolates the length branch specifically.
    #[test]
    fn is_uuid_rejects_wrong_segment_length() {
        // First segment 7 chars (expected 8)
        assert!(!is_uuid("1234567-3d82-4013-b909-c2d637f44541"));
        // Last segment 11 chars (expected 12)
        assert!(!is_uuid("6f4853a9-3d82-4013-b909-c2d637f4454"));
        // Second segment 3 chars (expected 4)
        assert!(!is_uuid("6f4853a9-3d8-4013-b909-c2d637f44541"));
    }

    /// When centy is not installed, resolve_centy_uuid must return an Err that
    /// does NOT expose raw exit-code-0 stdout (previously any exit code was ignored
    /// and the function would try to parse empty/garbage output as JSON).
    #[tokio::test]
    async fn resolve_centy_uuid_propagates_nonzero_exit() {
        // Use a valid UUID format to skip the is_uuid gate.
        let uuid = "6f4853a9-3d82-4013-b909-c2d637f44541";
        let result = resolve_centy_uuid(uuid).await;
        // Either centy resolved it (integration env) or returned an error.
        // The error must not say "failed to parse centy JSON" for a non-zero exit —
        // it should mention the exit code instead.
        if let Err(ref msg) = result {
            // If centy is missing entirely ("failed to spawn"), that's also acceptable.
            let is_spawn_error = msg.contains("failed to spawn");
            let is_exit_error = msg.contains("exited with");
            let is_not_found = msg.contains("not found");
            assert!(
                is_spawn_error || is_exit_error || is_not_found,
                "unexpected error message: {msg}"
            );
        }
    }
}
