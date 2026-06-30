use super::*;

#[test]
fn guard_goal_capture_and_active_goal() {
    let store = test_store("guard-goal");
    store
        .guard_capture_goal(
            "s1",
            Some("/repo"),
            "user-prompt-submit",
            "Make the gateway stream",
            None,
        )
        .unwrap();
    store
        .guard_capture_goal(
            "s1",
            Some("/repo"),
            "user-prompt-submit",
            "Add a /stream endpoint",
            None,
        )
        .unwrap();
    // A different session must not leak into s1's active goal.
    store
        .guard_capture_goal("s2", None, "session-start", "Unrelated work", None)
        .unwrap();

    let active = store.guard_active_goal("s1").unwrap().expect("active goal");
    assert_eq!(
        active.get("goal").and_then(|v| v.as_str()),
        Some("Add a /stream endpoint"),
        "most recent goal should win"
    );
    assert!(store.guard_active_goal("missing").unwrap().is_none());
}

#[test]
fn guard_block_streak_counts_blocks_and_caps() {
    let store = test_store("guard-streak");
    assert_eq!(store.guard_block_streak("s1").unwrap(), 0);

    store
        .guard_record_review(
            "s1",
            None,
            1,
            "claude",
            "codex",
            "allow",
            "looks good",
            None,
        )
        .unwrap();
    assert_eq!(
        store.guard_block_streak("s1").unwrap(),
        0,
        "allow must not count"
    );

    store
        .guard_record_review(
            "s1",
            None,
            2,
            "claude",
            "codex",
            "block",
            "incomplete",
            None,
        )
        .unwrap();
    store
        .guard_record_review(
            "s1",
            None,
            3,
            "claude",
            "codex",
            "block",
            "still incomplete",
            None,
        )
        .unwrap();
    assert_eq!(store.guard_block_streak("s1").unwrap(), 2);

    store
        .guard_record_review(
            "s1",
            None,
            4,
            "claude",
            "codex",
            "capped",
            "cap reached",
            None,
        )
        .unwrap();
    assert_eq!(
        store.guard_block_streak("s1").unwrap(),
        3,
        "capped also counts toward the streak so it won't re-block"
    );
}

#[test]
fn guard_enabled_defaults_true_and_toggles() {
    let store = test_store("guard-enabled");
    assert!(store.guard_enabled().unwrap(), "enabled by default");
    store.guard_set_enabled(false).unwrap();
    assert!(!store.guard_enabled().unwrap());
    store.guard_set_enabled(true).unwrap();
    assert!(store.guard_enabled().unwrap());
}

#[test]
fn guard_status_scopes_to_session() {
    let store = test_store("guard-status");
    store
        .guard_capture_goal("s1", None, "user-prompt-submit", "Goal one", None)
        .unwrap();
    store
        .guard_capture_goal("s2", None, "user-prompt-submit", "Goal two", None)
        .unwrap();

    let scoped = store.guard_status(Some("s1"), 20).unwrap();
    let goals = scoped.get("goals").and_then(|v| v.as_array()).unwrap();
    assert_eq!(goals.len(), 1);
    assert_eq!(
        goals[0].get("goal").and_then(|v| v.as_str()),
        Some("Goal one")
    );

    let all = store.guard_status(None, 20).unwrap();
    assert_eq!(
        all.get("goals").and_then(|v| v.as_array()).unwrap().len(),
        2
    );
}
