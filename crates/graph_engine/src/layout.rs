use crate::aoe::AoeNet;
use crate::critical_path::CriticalPathResult;
use petgraph::graph::NodeIndex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct NodeLayout {
    pub id: String,
    pub name: String,
    pub duration: f64,
    pub is_critical: bool,
    pub layer: usize,
    pub index_in_layer: usize,
}

#[derive(Debug, Clone)]
pub struct GraphLayout {
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<(usize, usize)>,
    pub layer_count: usize,
    pub max_nodes_in_layer: usize,
}

pub fn compute_layout(net: &AoeNet, result: &CriticalPathResult) -> GraphLayout {
    let topo = net
        .topological_order()
        .expect("graph should have been checked for cycles");

    let mut layers: HashMap<NodeIndex, usize> = HashMap::new();
    for &idx in &topo {
        let max_pred = net
            .predecessors(idx)
            .iter()
            .filter_map(|p| layers.get(p))
            .max()
            .copied()
            .unwrap_or(usize::MAX);
        layers.insert(idx, max_pred.wrapping_add(1));
    }

    let max_layer = layers.values().max().copied().unwrap_or(0);
    let mut layer_nodes: Vec<Vec<NodeIndex>> = vec![vec![]; max_layer + 1];
    for &idx in &topo {
        layer_nodes[layers[&idx]].push(idx);
    }

    let critical_set: HashSet<&str> = result.critical_path.iter().map(|s| s.as_str()).collect();

    let mut nodes = Vec::new();
    let mut node_idx_map = HashMap::new();

    let max_nodes = layer_nodes.iter().map(|l| l.len()).max().unwrap_or(1);

    for (layer, nodes_in_layer) in layer_nodes.iter().enumerate() {
        for (i, &idx) in nodes_in_layer.iter().enumerate() {
            let task = net.task(idx);
            node_idx_map.insert(idx, nodes.len());
            nodes.push(NodeLayout {
                id: task.id.clone(),
                name: task.name.clone(),
                duration: task.pert(),
                is_critical: critical_set.contains(task.id.as_str()),
                layer,
                index_in_layer: i,
            });
        }
    }

    let mut edges = Vec::new();
    for node in &nodes {
        let idx = net.index_map[&node.id];
        let from_global = node_idx_map[&idx];
        for succ in net.successors(idx) {
            let to_global = node_idx_map[&succ];
            edges.push((from_global, to_global));
        }
    }

    let actual_layers = layer_nodes.iter().filter(|l| !l.is_empty()).count();

    GraphLayout {
        nodes,
        edges,
        layer_count: actual_layers,
        max_nodes_in_layer: max_nodes,
    }
}
