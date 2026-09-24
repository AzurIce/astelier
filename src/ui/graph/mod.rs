//! 节点图页：dioxus-flow 画布 + 工具栏。
//!
//! 画布数据流：`graph.json` 首次载入 → 本地 `nodes`/`edges` 信号（flow 的
//! 单一事实来源）→ 任何变更（拖动/连线/删除/编辑节点内容）后整图落盘。
//! 页面按 graph.id 键控，切换图即重挂载。

mod nodes;

use crate::api::*;
use crate::app::AppState;
use crate::model::*;
use dioxus::prelude::*;
use dioxus_flow::prelude::*;
use dioxus_flow::{HandleKey, Id};

/// 生图节点内容区的固定宽度（高度自适应测量）
pub const GEN_NODE_WIDTH: f64 = 248.0;
/// 显示节点内容区的固定宽度
pub const DISPLAY_NODE_WIDTH: f64 = 232.0;

fn to_flow_nodes(g: &Graph) -> Vec<Node<NodeData>> {
    g.nodes
        .iter()
        .map(|n| {
            Node::with_data(n.id.clone(), n.kind().label(), (n.x, n.y), n.data.clone())
                .sides(Side::Left, Side::Right)
        })
        .collect()
}

fn to_flow_edges(g: &Graph) -> Vec<Edge> {
    g.edges
        .iter()
        .map(|e| {
            let mut fe = Edge::new(e.source.clone(), e.target.clone());
            fe.id = e.id.clone();
            fe.source_handle = e.source_handle.clone();
            fe.target_handle = e.target_handle.clone();
            fe
        })
        .collect()
}

fn from_flow_nodes(nodes: &[Node<NodeData>]) -> Vec<GraphNode> {
    nodes
        .iter()
        .map(|n| GraphNode {
            id: n.id.clone(),
            x: n.position.x,
            y: n.position.y,
            data: n.data.clone(),
        })
        .collect()
}

fn from_flow_edges(edges: &[Edge]) -> Vec<GraphEdge> {
    edges
        .iter()
        .map(|e| GraphEdge {
            id: e.id.clone(),
            source: e.source.clone(),
            target: e.target.clone(),
            source_handle: e.source_handle.clone(),
            target_handle: e.target_handle.clone(),
        })
        .collect()
}

/// 某节点的直接上游（Gen）节点 id 列表，按连线出现顺序
fn upstream_gens(edges: &[Edge], nodes: &[Node<NodeData>], node_id: &str) -> Vec<String> {
    let kind_of = |id: &str| nodes.iter().find(|n| n.id == id).map(|n| n.data.kind());
    edges
        .iter()
        .filter(|e| e.target == node_id)
        .filter(|e| kind_of(&e.source) == Some(NodeKind::Gen))
        .map(|e| e.source.clone())
        .collect()
}

/// 从生图节点发起生成：取节点当前装配内容发请求，批次插入全局列表。
fn run_node_action(
    state: AppState,
    nodes: Signal<Vec<Node<NodeData>>>,
    graph_id: String,
    node_id: String,
) {
    let Some(data) = nodes
        .peek()
        .iter()
        .find(|n| n.id == node_id)
        .and_then(|n| n.data.gen())
        .cloned()
    else {
        return;
    };
    if data.model_id.is_empty() {
        state.toast("请先在节点里选择模型", "error");
        return;
    }
    if data.prompt.trim().is_empty() {
        state.toast("请先填写 Prompt", "error");
        return;
    }
    let mut state = state;
    spawn(async move {
        match start_node_run(
            graph_id,
            node_id.clone(),
            NodeRunRequest {
                provider_id: data.provider_id.clone(),
                model_id: data.model_id.clone(),
                prompt: data.prompt.clone(),
                params: data.params.clone(),
                images: vec![],
                mask: None,
            },
        )
        .await
        {
            Ok(run) => state.runs.write().insert(0, run),
            Err(e) => state.toast(format!("发起失败：{e}"), "error"),
        }
    });
}

#[component]
pub fn GraphPage(state: AppState, graph: Graph) -> Element {
    let initial_nodes = graph.clone();
    let initial_edges = graph.clone();
    let mut nodes = use_signal(move || to_flow_nodes(&initial_nodes));
    let mut edges = use_signal(move || to_flow_edges(&initial_edges));
    let flow = use_flow_handle::<NodeData>();
    let mut connect_from = use_signal(|| None::<HandleKey>);
    let mut seq = use_signal(|| 0u64);
    // 生图节点参数折叠区的展开状态（按节点 id）。
    // dioxus-flow 按 2048 世界单位分块渲染，节点跨格会被卸载重挂，
    // 组件内局部状态会丢，因此提升到这里。
    let mut params_open: Signal<std::collections::HashSet<String>> = use_signal(Default::default);
    // 刚创建、正在播入场动画的节点（限时标记，260ms 后移除）。
    // 不能用 CSS class 翻转压制库动画：animation-name 从 none 切回本身
    // 就会重启动画（表现为松手即重播）。
    let mut entering: Signal<std::collections::HashSet<String>> = use_signal(Default::default);

    let cfg_at_create = state.config();

    // 整图落盘（meta 取自当前渲染的 props；闭包每次渲染重建，因此总是新鲜值）。
    // 捕获 graph 的克隆而非本体，避免把 prop 移进闭包导致后续借用失败。
    let graph_meta = graph.clone();
    let persist = move || {
        let nodes_ref = nodes.peek();
        let edges_ref = edges.peek();
        let g = Graph {
            id: graph_meta.id.clone(),
            title: graph_meta.title.clone(),
            group_id: graph_meta.group_id.clone(),
            nodes: from_flow_nodes(&nodes_ref),
            edges: from_flow_edges(&edges_ref),
            created_at: graph_meta.created_at,
            updated_at: crate::util::now_ms(),
        };
        state.patch_graph(g, |_| {});
    };

    // 连线合法性：只允许 生图 → 显示
    let is_valid_connection = move |conn: Connection| {
        let kind_of = |id: &str| {
            nodes
                .peek()
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.data.kind())
        };
        matches!(
            (kind_of(&conn.source), kind_of(&conn.target)),
            (Some(NodeKind::Gen), Some(NodeKind::Display))
        )
    };

    // ---- 视口中心附近的空闲落点（对角级联避让）----
    let free_spot = move |size: (f64, f64)| -> Point {
        let (center, occupied) = flow
            .core()
            .map(|core| {
                let rect = *core.container.peek();
                let center = core.client_to_flow(Point::new(
                    rect.x + rect.width / 2.0,
                    rect.y + rect.height / 2.0,
                ));
                (
                    center,
                    core.geoms.peek().iter().map(|g| g.rect).collect::<Vec<_>>(),
                )
            })
            .unwrap_or((Point::ZERO, Vec::new()));
        let base = Point::new(center.x - size.0 / 2.0, center.y - size.1 / 2.0);
        let mut pos = base;
        for step in 1..=64 {
            let free = !occupied.iter().any(|r| {
                r.x < pos.x + size.0 + 16.0
                    && pos.x - 16.0 < r.max_x()
                    && r.y < pos.y + size.1 + 16.0
                    && pos.y - 16.0 < r.max_y()
            });
            if free {
                break;
            }
            pos = base + Point::new(28.0 * step as f64, 28.0 * step as f64);
        }
        pos
    };

    // 每个 handler 各持一份 persist（渲染期克隆；persist 本体留在组件作用域）
    let persist_add_gen = persist.clone();
    let persist_add_disp = persist.clone();
    let persist_layout = persist.clone();
    let persist_connect = persist.clone();
    let persist_connect_end = persist.clone();
    let persist_drag = persist.clone();
    let persist_delete = persist.clone();
    let persist_node_view = persist.clone();
    let gid_delete = graph.id.clone();

    rsx! {
        div { class: "stage graph-page",
            div { class: "graph-toolbar",
                span { class: "graph-title", "{graph.display_title()}" }
                span { class: "graph-meta", "{graph.nodes.len()} 节点 · {graph.edges.len()} 连线" }
                div { class: "graph-toolbar-actions",
                    button {
                        class: "btn small",
                        onclick: move |_| {
                            let n = *seq.peek() + 1;
                            seq.set(n);
                            let id = format!("gen-{}-{n}", crate::util::now_ms());
                            let (provider_id, model_id) = cfg_at_create
                                .as_ref()
                                .and_then(|cfg| cfg.active())
                                .map(|p| {
                                    (p.id.clone(), p.models.first().cloned().unwrap_or_default())
                                })
                                .unwrap_or_default();
                            let pos = free_spot((GEN_NODE_WIDTH, 120.0));
                            let node = Node::with_data(
                                id.clone(),
                                NodeKind::Gen.label(),
                                (pos.x, pos.y),
                                NodeData::Gen(GenNodeData {
                                    provider_id,
                                    model_id,
                                    prompt: String::new(),
                                    params: Default::default(),
                                }),
                            )
                            .sides(Side::Left, Side::Right);
                            let mut node = node;
                            node.selected = true;
                            nodes.with_mut(|v| {
                                for n in v.iter_mut() {
                                    n.selected = false;
                                }
                                v.push(node);
                            });
                            entering.write().insert(id.clone());
                            let mut entering_sig = entering;
                            spawn(async move {
                                crate::util::delay(260).await;
                                entering_sig.write().remove(&id);
                            });
                            persist_add_gen();
                        },
                        crate::ui::icons::IconSparkles { size: 13 }
                        "生图节点"
                    }
                    button {
                        class: "btn small",
                        onclick: move |_| {
                            let n = *seq.peek() + 1;
                            seq.set(n);
                            let id = format!("disp-{}-{n}", crate::util::now_ms());
                            let pos = free_spot((DISPLAY_NODE_WIDTH, 140.0));
                            let node = Node::with_data(
                                id.clone(),
                                NodeKind::Display.label(),
                                (pos.x, pos.y),
                                NodeData::Display(DisplayNodeData::default()),
                            )
                            .sides(Side::Left, Side::Right);
                            let mut node = node;
                            node.selected = true;
                            nodes.with_mut(|v| {
                                for n in v.iter_mut() {
                                    n.selected = false;
                                }
                                v.push(node);
                            });
                            entering.write().insert(id.clone());
                            let mut entering_sig = entering;
                            spawn(async move {
                                crate::util::delay(260).await;
                                entering_sig.write().remove(&id);
                            });
                            persist_add_disp();
                        },
                        crate::ui::icons::IconImage { size: 13 }
                        "显示节点"
                    }
                    button {
                        class: "btn small",
                        title: "按连线方向自动排列",
                        onclick: move |_| {
                            flow.auto_layout(
                                &LayoutOptions::default().direction(LayoutDirection::LeftToRight),
                            );
                            let p = persist_layout.clone();
                            spawn(async move {
                                crate::util::delay(560).await;
                                p();
                            });
                        },
                        crate::ui::icons::IconWorkflow { size: 13 }
                        "整理"
                    }
                    button {
                        class: "btn small",
                        onclick: move |_| flow.fit_view(300),
                        "适应视图"
                    }
                    button {
                        class: "icon-btn",
                        title: "删除节点图",
                        onclick: move |_| {
                            let mut state = state;
                            let gid = gid_delete.clone();
                            spawn(async move {
                                let _ = delete_graph(gid.clone()).await;
                                state.graphs.with_mut(|v| v.retain(|g| g.id != gid));
                                if state.selected_graph_id() == gid {
                                    state.selected_graph.set(String::new());
                                }
                                state.toast("节点图已删除", "ok");
                            });
                        },
                        crate::ui::icons::IconTrash { size: 14 }
                    }
                }
            }

            div { class: "graph-canvas",
                Flow {
                    nodes,
                    edges,
                    fit_view: true,
                    drag_threshold: 4.0,
                    handle: flow,
                    is_valid_connection,
                    on_connect_start: move |key: HandleKey| connect_from.set(Some(key)),
                    on_connect: move |conn: Connection| {
                        let dup = edges
                            .peek()
                            .iter()
                            .any(|e| e.source == conn.source && e.target == conn.target);
                        if dup {
                            return;
                        }
                        edges.write().push(conn.into_edge());
                        persist_connect();
                    },
                    on_connect_end: move |end: ConnectEnd| {
                        let from = connect_from.cloned();
                        connect_from.set(None);
                        // 已成连线交给 on_connect；这里只处理「拖到空白处」
                        if end.connection.is_some() || from.is_none() {
                            return;
                        }
                        let key = from.unwrap();
                        if key.kind != HandleKind::Source {
                            return;
                        }
                        let from_gen = nodes
                            .peek()
                            .iter()
                            .any(|n| n.id == key.node && n.data.kind() == NodeKind::Gen);
                        if !from_gen {
                            return;
                        }
                        let n = *seq.peek() + 1;
                        seq.set(n);
                        let id = format!("disp-{}-{n}", crate::util::now_ms());
                        let node = Node::with_data(
                            id.clone(),
                            NodeKind::Display.label(),
                            (end.point.x, end.point.y),
                            NodeData::Display(DisplayNodeData::default()),
                        )
                        .sides(Side::Left, Side::Right);
                        let mut node = node;
                        node.selected = true;
                        edges.write().push(Edge::new(key.node.clone(), id.clone()));
                        nodes.with_mut(|v| {
                            for x in v.iter_mut() {
                                x.selected = false;
                            }
                            v.push(node);
                        });
                        entering.write().insert(id.clone());
                        let mut entering_sig = entering;
                        spawn(async move {
                            crate::util::delay(260).await;
                            entering_sig.write().remove(&id);
                        });
                        persist_connect_end();
                    },
                    on_node_drag_stop: move |_ids: Vec<Id>| {
                        persist_drag();
                    },
                    on_delete: move |req: DeleteRequest| {
                        edges.with_mut(|v| {
                            v.retain(|e| {
                                !req.edges.contains(&e.id)
                                    && !req.nodes.contains(&e.source)
                                    && !req.nodes.contains(&e.target)
                            });
                        });
                        nodes.with_mut(|v| v.retain(|n| !req.nodes.contains(&n.id)));
                        persist_delete();
                    },
                    node_view: move |ctx: NodeViewCtx<NodeData>| {
                        let gens = upstream_gens(&edges.peek(), &nodes.peek(), &ctx.node.id);
                        match &ctx.node.data {
                            NodeData::Gen(_) => {
                                // 内层 move 闭包只能拿每次调用新建的局部，不能
                                // move 外层闭包的捕获，否则 node_view 退化为 FnOnce
                                let gid = gid_view_of(&graph);
                                let nid_upd = ctx.node.id.clone();
                                let nid_run = ctx.node.id.clone();
                                let nid_po = ctx.node.id.clone();
                                let persist_upd = persist_node_view.clone();
                                let is_params_open = params_open.cloned().contains(&nid_upd);
                                let is_entering = entering.cloned().contains(&nid_upd);
                                rsx! {
                                    nodes::GenNodeView {
                                        state,
                                        ctx: ctx.clone(),
                                        graph_id: gid.clone(),
                                        params_open: is_params_open,
                                        on_params_open: move |v: bool| {
                                            let mut set = params_open.cloned();
                                            if v {
                                                set.insert(nid_po.clone());
                                            } else {
                                                set.remove(&nid_po);
                                            }
                                            params_open.set(set);
                                        },
                                        entering: is_entering,
                                        on_update: move |data: GenNodeData| {
                                            nodes.with_mut(|v| {
                                                if let Some(n) = v.iter_mut().find(|n| n.id == nid_upd) {
                                                    n.data = NodeData::Gen(data.clone());
                                                }
                                            });
                                            persist_upd();
                                        },
                                        on_run: move |_| {
                                            run_node_action(state, nodes, gid.clone(), nid_run.clone());
                                        },
                                    }
                                }
                            }
                            NodeData::Display(_) => {
                                let gid = gid_view_of(&graph);
                                let is_entering = entering.cloned().contains(&ctx.node.id);
                                rsx! {
                                    nodes::DisplayNodeView {
                                        state,
                                        ctx: ctx.clone(),
                                        graph_id: gid,
                                        upstream_gens: gens,
                                        entering: is_entering,
                                    }
                                }
                            }
                        }
                    },

                    Background { variant: BackgroundVariant::Dots }
                    Controls {}
                }
            }
        }
    }
}

/// 图 id（从当前渲染的 props 取；节点视图回调里每次调用时新建）
fn gid_view_of(graph: &Graph) -> String {
    graph.id.clone()
}
