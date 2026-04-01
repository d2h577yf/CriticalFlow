pub mod aoe;
pub mod critical_path;

#[cfg(feature = "python")]
mod python_api;

use std::path::Path;

pub use aoe::{AoeNet, Task, Dependency, ProjectData,AoeError};
pub use critical_path::{compute_critical_path, CriticalPathResult, TaskSchedule};

pub fn compute_critical_path_from_project(data: ProjectData) -> Result<CriticalPathResult, AoeError> {
    let net = data.into_net()?;
    compute_critical_path(&net)
}

pub fn compute_critical_path_from_json(json_str: &str) -> Result<CriticalPathResult, AoeError> {
    let data = ProjectData::from_json(json_str)
        .map_err(|e| AoeError::ParseError(e.to_string()))?;
    compute_critical_path_from_project(data)
}

pub fn compute_critical_path_from_file(path: &Path) -> Result<CriticalPathResult, AoeError> {
    let data = ProjectData::load_from_file(path)?;
    compute_critical_path_from_project(data)
}

pub fn compute_critical_path_as_json(json_str: &str) -> Result<String, AoeError> {
    let result = compute_critical_path_from_json(json_str)?;
    serde_json::to_string(&result).map_err(|e| AoeError::ParseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_preview_json() {
        let json = include_str!("../../../data/preview.json");
        let data = ProjectData::from_json(json).unwrap();

        assert_eq!(data.tasks.len(), 10);
        assert_eq!(data.dependencies.len(), 12);

        let t1 = &data.tasks[0];
        assert!((t1.pert() - 13.0 / 6.0).abs() < 1e-9);

        let network = data.into_net().unwrap();
        assert_eq!(network.graph.node_count(), 10);
        assert_eq!(network.graph.edge_count(), 12);
    }

    #[test]
    fn test_compute_critical_path_preview() {
        let json = include_str!("../../../data/preview.json");
        let result = compute_critical_path_from_json(json).unwrap();

        let expected_path = vec!["T1", "T2", "T3", "T5", "T8", "T9", "T10"];
        assert_eq!(result.critical_path, expected_path);
        assert!((result.project_duration - 21.0).abs() < 1e-9);

        let t4 = result
            .schedules
            .iter()
            .find(|s| s.task_id == "T4")
            .expect("T4 should exist");
        assert!(t4.slack > 0.0);
        assert!(!t4.is_critical);
    }

    #[test]
    fn test_compute_critical_path_json_output() {
        let json = include_str!("../../../data/preview.json");
        let output = compute_critical_path_as_json(json).unwrap();
        let parsed: CriticalPathResult = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed.schedules.len(), 10);
        assert_eq!(parsed.critical_path.first().unwrap(), "T1");
    }
}
