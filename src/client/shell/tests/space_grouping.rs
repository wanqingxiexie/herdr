use super::*;

fn workspace(number: usize, label: &str) -> ClientShellWorkspace {
    let mut workspace = snapshot().workspaces[0].clone();
    workspace.workspace_id = format!("ws_{number}");
    workspace.number = number;
    workspace.label = label.into();
    workspace.custom_label = true;
    workspace.focused = false;
    workspace.worktree = None;
    workspace
}

fn worktree(linked: bool, parent_workspace_id: Option<&str>) -> Option<ClientShellWorktree> {
    Some(ClientShellWorktree {
        key: "/repo/.git".into(),
        label: "repo".into(),
        is_linked_worktree: linked,
        parent_workspace_id: parent_workspace_id.map(str::to_owned),
    })
}

fn entries(snapshot: &ClientShellSnapshot) -> Vec<(String, bool, bool)> {
    super::super::render::workspace_entries(snapshot, &HashSet::new())
        .into_iter()
        .map(|entry| {
            (
                snapshot.workspaces[entry.index].workspace_id.clone(),
                entry.indented,
                entry.last_child,
            )
        })
        .collect()
}

fn nested_snapshot() -> ClientShellSnapshot {
    let mut projected = snapshot();
    let mut research = workspace(1, "research");
    research.worktree = worktree(false, None);
    let mut games = workspace(2, "games");
    games.worktree = worktree(false, None);
    let mut delta = workspace(3, "research/delta-rollout");
    delta.worktree = worktree(true, Some("ws_1"));
    let mut ctd = workspace(4, "games/ctd-rate");
    ctd.worktree = worktree(true, Some("ws_2"));
    let mut mm2 = workspace(5, "games/mm2-rate");
    mm2.worktree = worktree(true, Some("ws_2"));
    projected.workspaces = vec![research, games, delta, ctd, mm2];
    projected.focused_workspace_id = Some("ws_1".into());
    projected
}

#[test]
fn linked_worktrees_nest_under_their_own_parent_workspace() {
    let projected = nested_snapshot();

    assert_eq!(
        entries(&projected),
        vec![
            ("ws_1".into(), false, false),
            ("ws_3".into(), true, true),
            ("ws_2".into(), false, false),
            ("ws_4".into(), true, false),
            ("ws_5".into(), true, true),
        ]
    );
}

#[test]
fn linked_worktrees_without_provenance_fall_back_to_label_prefix() {
    let mut projected = nested_snapshot();
    // Endpoints that predate creation-time provenance send no parent id.
    for workspace in &mut projected.workspaces {
        if let Some(worktree) = workspace.worktree.as_mut() {
            worktree.parent_workspace_id = None;
        }
    }

    assert_eq!(
        entries(&projected),
        vec![
            ("ws_1".into(), false, false),
            ("ws_3".into(), true, true),
            ("ws_2".into(), false, false),
            ("ws_4".into(), true, false),
            ("ws_5".into(), true, true),
        ]
    );
}

#[test]
fn linked_worktree_with_unknown_parent_falls_back_to_label_prefix() {
    let mut projected = nested_snapshot();
    projected.workspaces[3]
        .worktree
        .as_mut()
        .unwrap()
        .parent_workspace_id = Some("ws_missing".into());

    assert_eq!(
        entries(&projected),
        vec![
            ("ws_1".into(), false, false),
            ("ws_3".into(), true, true),
            ("ws_2".into(), false, false),
            ("ws_4".into(), true, false),
            ("ws_5".into(), true, true),
        ]
    );
}

#[test]
fn parent_without_children_is_not_a_group() {
    let mut projected = nested_snapshot();
    projected
        .workspaces
        .retain(|workspace| workspace.workspace_id != "ws_4" && workspace.workspace_id != "ws_5");

    assert_eq!(
        entries(&projected),
        vec![
            ("ws_1".into(), false, false),
            ("ws_3".into(), true, true),
            ("ws_2".into(), false, false),
        ]
    );
    let games = projected
        .workspaces
        .iter()
        .find(|workspace| workspace.workspace_id == "ws_2")
        .unwrap();
    assert!(!super::super::sidebar::workspace_close_is_group(
        &projected, games
    ));
}

#[test]
fn collapsing_one_parent_hides_only_its_own_children() {
    let projected = nested_snapshot();
    let collapsed = HashSet::from(["ws_2".to_owned()]);
    let collapsed_entries = super::super::render::workspace_entries(&projected, &collapsed)
        .into_iter()
        .map(|entry| {
            (
                projected.workspaces[entry.index].workspace_id.clone(),
                entry.indented,
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        collapsed_entries,
        vec![
            ("ws_1".to_owned(), false),
            ("ws_3".to_owned(), true),
            ("ws_2".to_owned(), false),
        ]
    );
}

#[test]
fn collapsed_parent_status_aggregates_only_its_own_children() {
    let mut projected = nested_snapshot();
    projected.workspaces[2].agent_status = AgentStatus::Working;
    projected.workspaces[3].agent_status = AgentStatus::Blocked;
    projected.workspaces[4].agent_status = AgentStatus::Idle;
    let collapsed = HashSet::from(["ws_1".to_owned()]);

    let research = &projected.workspaces[0];
    assert_eq!(
        super::super::sidebar::displayed_workspace_status(&projected, research, &collapsed),
        AgentStatus::Working
    );

    let games = &projected.workspaces[1];
    assert_eq!(
        super::super::sidebar::displayed_workspace_status(&projected, games, &collapsed),
        AgentStatus::Idle
    );
}

#[test]
fn close_group_covers_only_the_parents_own_children() {
    let projected = nested_snapshot();
    let games = &projected.workspaces[1];
    assert!(super::super::sidebar::workspace_close_is_group(
        &projected, games
    ));

    let children = super::super::render::space_children(&projected);
    assert_eq!(children.get(&0).cloned(), Some(vec![2]));
    assert_eq!(children.get(&1).cloned(), Some(vec![3, 4]));
}
