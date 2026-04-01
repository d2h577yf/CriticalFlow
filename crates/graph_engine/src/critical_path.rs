use crate::aoe::{AoeNet, AoeError};
use petgraph::graph::NodeIndex;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct TaskSchedule {
    pub task_id: String,
    pub task_name: String,
    pub duration: f64,
    pub earliest_start: f64,
    pub earliest_finish: f64,
    pub latest_start: f64,
    pub latest_finish: f64,
    pub slack: f64,
    pub is_critical: bool,
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct CriticalPathResult {
    pub schedules: Vec<TaskSchedule>,
    pub critical_path: Vec<String>,
    pub project_duration: f64
}

pub fn compute_critical_path(net: &AoeNet) -> Result<CriticalPathResult, AoeError> {
    let topo_order = net.topological_order()?;

    let mut es: HashMap<NodeIndex,f64> = HashMap::new();
    let mut ef: HashMap<NodeIndex,f64> = HashMap::new();

    for &idx in &topo_order {
        let dur = net.task(idx).pert();

        let earliest_start = net
            .predecessors(idx)
            .iter()
            .map(|&pred| ef[&pred])
            .fold(0.0_f64,f64::max);

        es.insert(idx,earliest_start);
        ef.insert(idx,earliest_start + dur);
    };

    let project_duration = ef.values().copied().fold(0.0_f64,f64::max);

    let mut ls: HashMap<NodeIndex,f64> = HashMap::new();
    let mut lf: HashMap<NodeIndex,f64> = HashMap::new();

    for &idx in topo_order.iter().rev() {
        let dur = net.task(idx).pert();

        let successors = net.successors(idx);

        let latest_finish = if successors.is_empty() {
            project_duration
        } else {
            successors
                .iter()
                .map(|&succ| ls[&succ])
                .fold(f64::INFINITY,f64::min)
        };
        lf.insert(idx,latest_finish);
        ls.insert(idx,latest_finish-dur);
    };

    let mut schedules = Vec::new();
    let mut critical_path = Vec::new();

    for &idx in &topo_order {
       let task = net.task(idx);
        let dur = task.pert();
        let e_s = es[&idx];
        let e_f = ef[&idx];
        let l_s = ls[&idx];
        let l_f = lf[&idx];
        let slack = l_s - e_s;
        let is_critical = slack.abs() < 1e-9;

        if is_critical {
            critical_path.push(task.id.clone());
        }

        schedules.push(TaskSchedule {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            duration: dur,
            earliest_start: e_s,
            earliest_finish: e_f,
            latest_start: l_s,
            latest_finish: l_f,
            slack,
            is_critical
        });
    };

    Ok(CriticalPathResult{
        schedules,
        critical_path,
        project_duration,
    })
}

