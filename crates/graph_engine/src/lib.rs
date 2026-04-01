pub mod aoe;
pub mod critical_path;

pub use aoe::{AoeNet, Task, Dependency, ProjectData,AoeError};
pub use critical_path::{compute_critical_path, CriticalPathResult, TaskSchedule};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_preview_json() {
        let json = include_str!("../../../data/preview.json");
        let data = ProjectData::from_json(json).unwrap();

        assert_eq!(data.tasks.len(), 10);
        assert_eq!(data.dependencies.len(), 12);

        // T1 的 PERT 期望值: (1 + 4*2 + 4) / 6 = 13/6 ≈ 2.167
        let t1 = &data.tasks[0];
        assert!((t1.pert() - 13.0 / 6.0).abs() < 1e-9);

        // 能成功建图
        let network = data.into_net().unwrap();
        assert_eq!(network.graph.node_count(), 10);
        assert_eq!(network.graph.edge_count(), 12);
    }
}
