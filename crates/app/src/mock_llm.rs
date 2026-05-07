use graph_engine::{
    CriticalPathResult, Dependency, ProjectData, Task, compute_critical_path_from_project,
};

/// Returns (project_data, critical_path_result, summary_text)
pub fn generate_mock_project(input: &str) -> (ProjectData, CriticalPathResult, String) {
    let lower = input.to_lowercase();

    let (project_type_name, tasks, deps) = if lower.contains("商城")
        || lower.contains("电商")
        || lower.contains("购物")
        || lower.contains("商店")
        || lower.contains("shop")
        || lower.contains("store")
        || lower.contains("ecommerce")
    {
        ("商城系统", make_mall_tasks(), make_mall_deps())
    } else if lower.contains("博客")
        || lower.contains("blog")
        || lower.contains("cms")
        || lower.contains("内容管理")
        || lower.contains("content")
    {
        ("博客/CMS系统", make_blog_tasks(), make_blog_deps())
    } else if lower.contains("后台")
        || lower.contains("admin")
        || lower.contains("管理系统")
        || lower.contains("dashboard")
    {
        ("后台管理系统", make_admin_tasks(), make_admin_deps())
    } else {
        ("通用软件项目", make_default_tasks(), make_default_deps())
    };

    let data = ProjectData {
        tasks,
        dependencies: deps,
    };

    let result = compute_critical_path_from_project(data.clone())
        .expect("generated project data should be valid");

    let n = data.tasks.len();
    let path_str = result.critical_path.join(" → ");
    let summary = format!(
        "解析结果：识别为 {}\n共 {} 个任务，关键路径为：{}\n项目预计总工期：{:.1} 天",
        project_type_name, n, path_str, result.project_duration
    );

    (data, result, summary)
}

// ── 商城/电商 ────────────────────────────────────────────────────────

fn make_mall_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "T1".into(),
            name: "需求分析".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T2".into(),
            name: "技术选型".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
        Task {
            id: "T3".into(),
            name: "数据库设计".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T4".into(),
            name: "用户系统".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 8.0,
        },
        Task {
            id: "T5".into(),
            name: "商品管理".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 8.0,
        },
        Task {
            id: "T6".into(),
            name: "购物车".into(),
            duration_optimistic: 2.0,
            duration_normal: 4.0,
            duration_pessimistic: 6.0,
        },
        Task {
            id: "T7".into(),
            name: "订单系统".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 8.0,
        },
        Task {
            id: "T8".into(),
            name: "支付集成".into(),
            duration_optimistic: 2.0,
            duration_normal: 4.0,
            duration_pessimistic: 7.0,
        },
        Task {
            id: "T9".into(),
            name: "前端页面".into(),
            duration_optimistic: 4.0,
            duration_normal: 7.0,
            duration_pessimistic: 10.0,
        },
        Task {
            id: "T10".into(),
            name: "测试".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T11".into(),
            name: "部署上线".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
    ]
}

fn make_mall_deps() -> Vec<Dependency> {
    vec![
        Dependency {
            from: "T1".into(),
            to: "T2".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T3".into(),
        },
        Dependency {
            from: "T3".into(),
            to: "T4".into(),
        },
        Dependency {
            from: "T3".into(),
            to: "T5".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T7".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T6".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T7".into(),
        },
        Dependency {
            from: "T6".into(),
            to: "T8".into(),
        },
        Dependency {
            from: "T7".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T8".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T9".into(),
            to: "T11".into(),
        },
        Dependency {
            from: "T10".into(),
            to: "T11".into(),
        },
    ]
}

// ── 博客/CMS ─────────────────────────────────────────────────────────

fn make_blog_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "T1".into(),
            name: "需求分析".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
        Task {
            id: "T2".into(),
            name: "数据库设计".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T3".into(),
            name: "用户认证".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T4".into(),
            name: "文章管理".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 7.0,
        },
        Task {
            id: "T5".into(),
            name: "评论系统".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T6".into(),
            name: "分类标签".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
        Task {
            id: "T7".into(),
            name: "搜索功能".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T8".into(),
            name: "管理后台".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 8.0,
        },
        Task {
            id: "T9".into(),
            name: "前端展示".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 7.0,
        },
        Task {
            id: "T10".into(),
            name: "部署".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
    ]
}

fn make_blog_deps() -> Vec<Dependency> {
    vec![
        Dependency {
            from: "T1".into(),
            to: "T2".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T3".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T4".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T6".into(),
        },
        Dependency {
            from: "T3".into(),
            to: "T8".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T5".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T7".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T6".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T8".into(),
        },
        Dependency {
            from: "T7".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T8".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T9".into(),
            to: "T10".into(),
        },
    ]
}

// ── 后台管理 ─────────────────────────────────────────────────────────

fn make_admin_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "T1".into(),
            name: "需求分析".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
        Task {
            id: "T2".into(),
            name: "权限系统".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T3".into(),
            name: "数据表设计".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T4".into(),
            name: "CRUD生成".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 7.0,
        },
        Task {
            id: "T5".into(),
            name: "数据可视化".into(),
            duration_optimistic: 2.0,
            duration_normal: 4.0,
            duration_pessimistic: 6.0,
        },
        Task {
            id: "T6".into(),
            name: "文件管理".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T7".into(),
            name: "日志审计".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T8".into(),
            name: "搜索筛选".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T9".into(),
            name: "前端框架".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 7.0,
        },
        Task {
            id: "T10".into(),
            name: "部署".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
    ]
}

fn make_admin_deps() -> Vec<Dependency> {
    vec![
        Dependency {
            from: "T1".into(),
            to: "T2".into(),
        },
        Dependency {
            from: "T1".into(),
            to: "T3".into(),
        },
        Dependency {
            from: "T3".into(),
            to: "T4".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T4".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T6".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T7".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T8".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T7".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T6".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T8".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T9".into(),
            to: "T10".into(),
        },
    ]
}

// ── 通用软件项目 (default) ───────────────────────────────────────────

fn make_default_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "T1".into(),
            name: "需求分析与评审".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T2".into(),
            name: "系统架构设计".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T3".into(),
            name: "设计数据库".into(),
            duration_optimistic: 2.0,
            duration_normal: 3.0,
            duration_pessimistic: 5.0,
        },
        Task {
            id: "T4".into(),
            name: "搭建容器化开发环境".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
        Task {
            id: "T5".into(),
            name: "写后端".into(),
            duration_optimistic: 3.0,
            duration_normal: 5.0,
            duration_pessimistic: 8.0,
        },
        Task {
            id: "T6".into(),
            name: "写前端".into(),
            duration_optimistic: 3.0,
            duration_normal: 4.0,
            duration_pessimistic: 7.0,
        },
        Task {
            id: "T7".into(),
            name: "编写API文档".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
        },
        Task {
            id: "T8".into(),
            name: "前后端联调测试".into(),
            duration_optimistic: 2.0,
            duration_normal: 4.0,
            duration_pessimistic: 6.0,
        },
        Task {
            id: "T9".into(),
            name: "性能压测与Bug修复".into(),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 4.0,
        },
        Task {
            id: "T10".into(),
            name: "生产环境部署上线".into(),
            duration_optimistic: 1.0,
            duration_normal: 1.0,
            duration_pessimistic: 2.0,
        },
    ]
}

fn make_default_deps() -> Vec<Dependency> {
    vec![
        Dependency {
            from: "T1".into(),
            to: "T2".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T3".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T4".into(),
        },
        Dependency {
            from: "T2".into(),
            to: "T6".into(),
        },
        Dependency {
            from: "T3".into(),
            to: "T5".into(),
        },
        Dependency {
            from: "T4".into(),
            to: "T5".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T7".into(),
        },
        Dependency {
            from: "T5".into(),
            to: "T8".into(),
        },
        Dependency {
            from: "T6".into(),
            to: "T8".into(),
        },
        Dependency {
            from: "T8".into(),
            to: "T9".into(),
        },
        Dependency {
            from: "T7".into(),
            to: "T10".into(),
        },
        Dependency {
            from: "T9".into(),
            to: "T10".into(),
        },
    ]
}
