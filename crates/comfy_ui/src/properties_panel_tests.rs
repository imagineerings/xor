use crate::graph_tests::fixture_model;
use crate::properties_panel::{
    GraphNodePropertyKind, graph_node_properties_with_value, graph_node_property_descriptors,
    parse_graph_node_property_value,
};
use crate::{GraphPropertiesPanel, GraphWorkspaceItem, open_for_graph_node};
use comfy_runtime::{GraphIdentifier, GraphNode, GraphNodeMode, GraphPaletteColor, GraphPoint};
use gpui::{AppContext as _, Focusable as _, TestAppContext};
use project::{FakeFs, Project};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel as _},
};

fn init_properties_test(cx: &mut TestAppContext) {
    let file_system = FakeFs::new(cx.executor());
    cx.update(|cx| {
        cx.set_global(db::AppDatabase::test_new());
        let settings_store = settings::SettingsStore::test(cx);
        cx.set_global(settings_store);
        <dyn fs::Fs>::set_global(file_system, cx);
        theme_settings::init(theme::LoadThemes::JustBase, cx);
        workspace::AppState::test(cx);
        crate::init(cx);
    });
    cx.run_until_parked();
}

#[gpui::test(seed = 16024)]
async fn properties_panel_is_a_right_dock_bound_to_canonical_graph_commands(
    cx: &mut TestAppContext,
) {
    init_properties_test(cx);
    let file_system = FakeFs::new(cx.executor());
    let project = Project::test(file_system, [], cx).await;
    let (workspace, window) =
        cx.add_window_view(|window, cx| Workspace::test_new(project, window, cx));
    let (graph, panel) = workspace.update_in(window, |workspace, window, cx| {
        let workspace_handle = cx.entity().downgrade();
        let graph = cx.new(|cx| {
            GraphWorkspaceItem::new(
                fixture_model().expect("create native graph properties fixture"),
                workspace_handle.clone(),
                cx,
            )
        });
        workspace.add_item_to_active_pane(Box::new(graph.clone()), None, true, window, cx);
        let panel =
            cx.new(|cx| GraphPropertiesPanel::test_new(workspace_handle.clone(), window, cx));
        workspace.add_panel(panel.clone(), window, cx);
        (graph, panel)
    });
    window.run_until_parked();
    workspace.update_in(window, |workspace, window, cx| {
        open_for_graph_node(
            workspace,
            graph.clone(),
            GraphIdentifier::from("source"),
            window,
            cx,
        )
        .expect("open authoritative native graph properties panel");
    });
    window.run_until_parked();

    workspace.update_in(window, |workspace, window, cx| {
        assert_eq!(panel.read(cx).position(window, cx), DockPosition::Right);
        assert!(workspace.right_dock().read(cx).is_open());
        assert!(panel.focus_handle(cx).contains_focused(window, cx));
    });
    assert_eq!(
        panel.read_with(window, |panel, _| panel.target_for_test()),
        Some(GraphIdentifier::from("source"))
    );

    panel.update_in(window, |panel, window, cx| {
        panel.set_title_for_test("Production Source", window, cx);
        panel.set_mode_for_test(GraphNodeMode::OnTrigger, cx);
        panel.set_color_for_test(Some(GraphPaletteColor::Blue), cx);
        panel.set_properties_for_test(r#"{"priority":7,"owner":"runtime"}"#, window, cx);
    });
    window.run_until_parked();
    graph.read_with(window, |graph, _| {
        let node = graph
            .model()
            .document()
            .expect("editable properties graph")
            .active_graph()
            .expect("active properties graph")
            .nodes
            .get(&GraphIdentifier::from("source"))
            .expect("properties target remains present");
        assert_eq!(node.title, "Production Source");
        assert_eq!(node.mode, GraphNodeMode::OnTrigger);
        assert_eq!(
            node.color.as_deref(),
            Some(GraphPaletteColor::Blue.node_header())
        );
        assert_eq!(
            node.source_fields
                .get("properties")
                .and_then(serde_json::Value::as_object)
                .and_then(|properties| properties.get("priority")),
            Some(&serde_json::Value::from(7))
        );
    });

    assert!(panel.update(window, |panel, cx| panel.delete_for_test(cx)));
    window.run_until_parked();
    graph.read_with(window, |graph, _| {
        assert!(
            !graph
                .model()
                .document()
                .expect("editable properties graph")
                .active_graph()
                .expect("active properties graph")
                .nodes
                .contains_key(&GraphIdentifier::from("source"))
        );
    });
    assert!(!panel.update(window, |panel, cx| panel.delete_for_test(cx)));
    assert!(panel.read_with(window, |panel, cx| {
        panel
            .availability_for_test(cx)
            .is_some_and(|message| message.contains("no longer exists"))
    }));

    graph.update(window, |graph, cx| {
        assert!(graph.model.undo().expect("undo canonical node deletion"));
        cx.notify();
    });
    window.run_until_parked();
    assert!(graph.read_with(window, |graph, _| {
        graph
            .model()
            .document()
            .expect("editable properties graph")
            .active_graph()
            .expect("active properties graph")
            .nodes
            .contains_key(&GraphIdentifier::from("source"))
    }));
}

#[test]
fn sim_loads_properties_panel_through_the_workspace_panel_owner() {
    const SIM_SOURCE: &str = include_str!("../../sim/src/sim.rs");
    assert!(SIM_SOURCE.contains("comfy_ui::GraphPropertiesPanel::load"));
    assert!(SIM_SOURCE.contains("add_panel_when_ready(\n                graph_properties_panel"));
}

#[test]
fn typed_property_adapter_is_the_shared_bounded_properties_info_owner() {
    let mut node = GraphNode::new(
        GraphIdentifier::from("typed-properties"),
        "NativeFixture",
        "Typed properties",
        GraphPoint::ZERO,
    );
    node.source_fields.insert(
        "properties".to_owned(),
        serde_json::json!({
            "enabled": true,
            "quality": "balanced",
            "scheduler": 0,
            "inferred_bool": false,
            "inferred_number": 3.5,
            "inferred_text": "native",
            "untyped": {"preserved": true}
        }),
    );
    node.source_fields.insert(
        "properties_info".to_owned(),
        serde_json::json!([
            {"name":"enabled","label":"Enabled","type":"boolean","default_value":false},
            {
                "name":"quality",
                "label":"Quality",
                "type":"combo",
                "values":["fast","balanced","high"]
            },
            {
                "name":"scheduler",
                "label":"Scheduler",
                "type":"enum",
                "values":{"Normal":0,"High":1}
            }
        ]),
    );

    let descriptors = graph_node_property_descriptors(&node)
        .expect("parse bounded native node property descriptors");
    assert_eq!(descriptors.len(), 7);
    assert_eq!(descriptors[0].key, "enabled");
    assert_eq!(descriptors[0].kind, GraphNodePropertyKind::Boolean);
    assert_eq!(descriptors[1].key, "quality");
    assert!(matches!(
        descriptors[1].kind,
        GraphNodePropertyKind::Choice { .. }
    ));
    assert_eq!(descriptors[2].key, "scheduler");
    assert!(matches!(
        descriptors[2].kind,
        GraphNodePropertyKind::Choice { .. }
    ));
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.key == "inferred_bool")
            .map(|descriptor| &descriptor.kind),
        Some(&GraphNodePropertyKind::Boolean)
    );
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.key == "inferred_number")
            .map(|descriptor| &descriptor.kind),
        Some(&GraphNodePropertyKind::Number)
    );
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.key == "inferred_text")
            .map(|descriptor| &descriptor.kind),
        Some(&GraphNodePropertyKind::Text)
    );
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.key == "untyped")
            .map(|descriptor| &descriptor.kind),
        Some(&GraphNodePropertyKind::Json)
    );
    let parsed = parse_graph_node_property_value(&descriptors[1], "high")
        .expect("parse declared combo value");
    assert_eq!(parsed, serde_json::Value::String("high".to_owned()));
    assert!(parse_graph_node_property_value(&descriptors[1], "unsupported").is_err());
    assert_eq!(
        parse_graph_node_property_value(&descriptors[2], "High")
            .expect("parse object-mapped enum label"),
        serde_json::json!(1)
    );
    let updated = graph_node_properties_with_value(&node, "quality", parsed)
        .expect("update typed property through shared adapter");
    assert_eq!(updated.get("quality"), Some(&serde_json::json!("high")));
    assert!(
        graph_node_properties_with_value(&node, "quality", serde_json::json!("unsupported"))
            .is_err()
    );

    let mut duplicate_value_node = node;
    duplicate_value_node.source_fields.insert(
        "properties_info".to_owned(),
        serde_json::json!([
            {"name":"scheduler","type":"enum","values":{"One":1,"Also One":1}}
        ]),
    );
    assert!(graph_node_property_descriptors(&duplicate_value_node).is_err());
}
