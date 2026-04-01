use petgraph::graph::{DiGraph,NodeIndex};
use petgraph::algo::toposort;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,

    #[serde(rename = "duration_opt")]
    pub duration_optimistic: f64,

    #[serde(rename = "duration_norm")]
    pub duration_normal: f64,

    #[serde(rename = "duration_pess")]
    pub duration_pessimistic: f64,
}

impl Task {
    pub fn pert(&self) -> f64 {
        (self.duration_optimistic + 4.0 * self.duration_normal + self.duration_pessimistic) / 6.0
    }

    pub fn std_dev(&self) -> f64 {
        (self.duration_optimistic - self.duration_pessimistic) / 6.0
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Dependency {
    pub from : String,
    pub to : String,
}

#[derive(Debug, thiserror::Error)]
pub enum AoeError {
    #[error("cycle detected at task: {node}")]
    CycleDetected { node: String },

    #[error("unknown task referenced in dependency: {0}")]
    UnknownTask(String),

    #[error("failed to read file: {0}")]
    IoError(String),

    #[error("failed to parse JSON: {0}")]
    ParseError(String),
}

pub struct AoeNet {
    pub graph : DiGraph<Task,()>,
    pub index_map : HashMap<String,NodeIndex>
}

impl AoeNet {
    fn build(tasks : Vec<Task>,deps : Vec<Dependency>) -> Result<Self,AoeError> {
        let mut graph = DiGraph::new();
        let mut index_map = HashMap::new();

        for task in tasks {
            let id = task.id.clone();
            let idx = graph.add_node(task);
            index_map.insert(id,idx);
        };

        for dep in &deps {
            let &from_idx = index_map
                .get(&dep.from)
                .ok_or_else(|| AoeError::UnknownTask(dep.from.clone()))?;
            let &to_idx = index_map
                .get((&dep.to))
                .ok_or_else(|| AoeError::UnknownTask(dep.to.clone()))?;
        };

        toposort(&graph,None).map_err(|cycle| AoeError::CycleDetected {
           node: graph[cycle.node_id()].id.clone(),
        })?;

        Ok(Self {
            graph,
            index_map,
        })
    }

    pub fn topological_order(&self) -> Result<Vec<NodeIndex>,AoeError> {
        toposort(&self.graph,None).map_err(|cycle| AoeError::CycleDetected {
            node: self.graph[cycle.node_id()].id.clone(),
        })
    }

    pub fn predecessors(&self,idx : NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(idx,Direction::Incoming)
            .collect()
    }

    pub fn successors(&self,idx : NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(idx,Direction::Outgoing)
            .collect()
    }

    pub fn task(&self,idx : NodeIndex) -> &Task {
        &self.graph[idx]
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub tasks: Vec<Task>,
    pub dependencies: Vec<Dependency>,
}

impl ProjectData {
    pub fn from_json(json_str: &str) -> Result<Self,serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn load_from_file(path: &std::path::Path) -> Result<Self,AoeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AoeError::IoError(e.to_string()))?;
        Self::from_json(&content)
            .map_err(|e| AoeError::ParseError(e.to_string()))
    }

    pub fn into_net(self) -> Result<AoeNet,AoeError> {
        AoeNet::build(self.tasks,self.dependencies)
    }
}
