use eframe::{Frame, egui};
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use graph_engine::{
    CriticalPathResult, Dependency, GraphLayout, NodeLayout, ProjectData, Task,
    compute_critical_path_from_json, compute_critical_path_from_project, compute_layout,
};

#[path = "mock_llm.rs"]
mod mock_llm;

// ── Drawing constants ──────────────────────────────────────────────

const NODE_W: f32 = 150.0;
const NODE_H: f32 = 58.0;
const H_GAP: f32 = 230.0;
const V_GAP: f32 = 95.0;
const MARGIN: f32 = 60.0;
const VIEW_PADDING: f32 = 36.0;

const GRAPH_ZOOM_MIN: f32 = 0.35;
const GRAPH_ZOOM_MAX: f32 = 2.75;
const GRAPH_ZOOM_STEP: f32 = 1.18;
const GRAPH_TRANSITION_SECONDS: f32 = 0.75;

const CRITICAL_FILL: Color32 = Color32::from_rgb(255, 215, 190);
const CRITICAL_BORDER: Color32 = Color32::from_rgb(210, 70, 60);
const CRITICAL_TEXT: Color32 = Color32::from_rgb(150, 35, 30);

const NORMAL_FILL: Color32 = Color32::from_rgb(210, 225, 250);
const NORMAL_BORDER: Color32 = Color32::from_rgb(105, 135, 185);
const NORMAL_TEXT: Color32 = Color32::from_rgb(40, 48, 65);

const EDGE_CRITICAL: Color32 = Color32::from_rgb(210, 70, 60);
const EDGE_NORMAL: Color32 = Color32::from_rgb(165, 165, 170);

const DURATION_TEXT: Color32 = Color32::from_rgb(115, 115, 120);

// ── App types ──────────────────────────────────────────────────────

pub enum LlmMessage {
    Thinking,
    Done(String),
    Error(String),
}

pub struct ChatMessage {
    pub is_user: bool,
    pub content: String,
}

#[derive(PartialEq, Clone, Copy)]
enum RunningMode {
    Idle,
    Running,
}

struct GraphTransition {
    from: GraphLayout,
    started_at: Instant,
    from_zoom: f32,
    from_pan: Vec2,
}

struct GraphTransitionFrame {
    from: GraphLayout,
    raw_progress: f32,
    eased_progress: f32,
    from_zoom: f32,
    from_pan: Vec2,
}

struct AddNodeForm {
    name: String,
    duration_optimistic: f64,
    duration_normal: f64,
    duration_pessimistic: f64,
    predecessor_ids: HashSet<String>,
    successor_ids: HashSet<String>,
    error: Option<String>,
}

impl Default for AddNodeForm {
    fn default() -> Self {
        Self {
            name: String::from("新任务"),
            duration_optimistic: 1.0,
            duration_normal: 2.0,
            duration_pessimistic: 3.0,
            predecessor_ids: HashSet::new(),
            successor_ids: HashSet::new(),
            error: None,
        }
    }
}

pub struct InputWindow {
    user_input: String,
    chat_history: Vec<ChatMessage>,

    mode: RunningMode,
    status_msg: String,

    llm_sender: Option<Sender<String>>,
    result_receiver: Option<Receiver<LlmMessage>>,

    // ── Graph state ────────────────────────────────────────────
    project_data: Option<ProjectData>,
    critical_path: Option<CriticalPathResult>,
    graph_layout: Option<GraphLayout>,

    graph_zoom: f32,
    graph_pan: Vec2,
    graph_fit_pending: bool,
    graph_transition: Option<GraphTransition>,

    selected_node_id: Option<String>,
    context_node_id: Option<String>,
    show_add_node_dialog: bool,
    add_node_form: AddNodeForm,
}

impl Default for InputWindow {
    fn default() -> Self {
        InputWindow {
            user_input: String::new(),
            chat_history: Vec::new(),
            mode: RunningMode::Idle,
            status_msg: String::from("就绪"),
            llm_sender: None,
            result_receiver: None,
            project_data: None,
            critical_path: None,
            graph_layout: None,
            graph_zoom: 1.0,
            graph_pan: Vec2::ZERO,
            graph_fit_pending: true,
            graph_transition: None,
            selected_node_id: None,
            context_node_id: None,
            show_add_node_dialog: false,
            add_node_form: AddNodeForm::default(),
        }
    }
}

impl InputWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::set_font_custom(&cc.egui_ctx);
        let mut window = Self::default();
        window.load_demo_data();
        window
    }

    fn set_font_custom(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_chinese_font".to_owned(),
            Arc::from(egui::FontData::from_static(include_bytes!(
                "../../../assets/SourceHanSansCN-Medium.otf"
            ))),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_chinese_font".to_owned());
        ctx.set_fonts(fonts);
    }

    fn load_demo_data(&mut self) {
        let json = include_str!("../../../data/preview.json");
        if let Ok(data) = serde_json::from_str::<ProjectData>(json) {
            if let Ok(result) = compute_critical_path_from_json(json) {
                if let Ok(net) = data.clone().into_net() {
                    self.set_graph_layout(compute_layout(&net, &result), false);
                    self.project_data = Some(data);
                    self.critical_path = Some(result);
                }
            }
        }
    }

    fn set_graph_layout(&mut self, layout: GraphLayout, animate: bool) {
        if animate {
            if let Some(previous) = &self.graph_layout {
                self.graph_transition = Some(GraphTransition {
                    from: previous.clone(),
                    started_at: Instant::now(),
                    from_zoom: self.graph_zoom,
                    from_pan: self.graph_pan,
                });
            }
        } else {
            self.graph_transition = None;
        }

        self.graph_layout = Some(layout);
        self.graph_fit_pending = true;
    }

    fn commit_project_data(&mut self, data: ProjectData, animate: bool) -> Result<(), String> {
        let result = compute_critical_path_from_project(data.clone()).map_err(|e| e.to_string())?;
        let net = data.clone().into_net().map_err(|e| e.to_string())?;

        self.set_graph_layout(compute_layout(&net, &result), animate);
        self.project_data = Some(data);
        self.critical_path = Some(result);
        Ok(())
    }

    fn open_add_node_dialog(&mut self, predecessors: &[String], successors: &[String]) {
        self.add_node_form = AddNodeForm::default();
        self.add_node_form
            .predecessor_ids
            .extend(predecessors.iter().cloned());
        self.add_node_form
            .successor_ids
            .extend(successors.iter().cloned());
        self.show_add_node_dialog = true;
    }

    fn next_task_id(data: &ProjectData) -> String {
        let used: HashSet<&str> = data.tasks.iter().map(|task| task.id.as_str()).collect();
        for i in 1.. {
            let candidate = format!("T{}", i);
            if !used.contains(candidate.as_str()) {
                return candidate;
            }
        }

        unreachable!("usize range should always provide a free task id")
    }

    fn add_manual_node(&mut self) {
        let name = self.add_node_form.name.trim().to_string();
        if name.is_empty() {
            self.add_node_form.error = Some(String::from("任务名不能为空"));
            return;
        }

        let opt = self.add_node_form.duration_optimistic;
        let normal = self.add_node_form.duration_normal;
        let pess = self.add_node_form.duration_pessimistic;

        if !opt.is_finite() || !normal.is_finite() || !pess.is_finite() {
            self.add_node_form.error = Some(String::from("工期必须是有效数字"));
            return;
        }

        if opt <= 0.0 || normal <= 0.0 || pess <= 0.0 {
            self.add_node_form.error = Some(String::from("工期必须大于 0"));
            return;
        }

        if !(opt <= normal && normal <= pess) {
            self.add_node_form.error = Some(String::from("请保持乐观 <= 最可能 <= 悲观"));
            return;
        }

        if !self
            .add_node_form
            .predecessor_ids
            .is_disjoint(&self.add_node_form.successor_ids)
        {
            self.add_node_form.error = Some(String::from("同一任务不能同时作为前置和后续"));
            return;
        }

        let mut next_data = self.project_data.clone().unwrap_or(ProjectData {
            tasks: Vec::new(),
            dependencies: Vec::new(),
        });

        let new_id = Self::next_task_id(&next_data);
        let existing_ids: HashSet<String> =
            next_data.tasks.iter().map(|task| task.id.clone()).collect();

        next_data.tasks.push(Task {
            id: new_id.clone(),
            name,
            duration_optimistic: opt,
            duration_normal: normal,
            duration_pessimistic: pess,
        });

        for from in &self.add_node_form.predecessor_ids {
            if existing_ids.contains(from) {
                next_data.dependencies.push(Dependency {
                    from: from.clone(),
                    to: new_id.clone(),
                });
            }
        }

        for to in &self.add_node_form.successor_ids {
            if existing_ids.contains(to) {
                next_data.dependencies.push(Dependency {
                    from: new_id.clone(),
                    to: to.clone(),
                });
            }
        }

        next_data
            .dependencies
            .sort_by(|a, b| a.from.cmp(&b.from).then_with(|| a.to.cmp(&b.to)));
        next_data
            .dependencies
            .dedup_by(|a, b| a.from == b.from && a.to == b.to);

        match self.commit_project_data(next_data, true) {
            Ok(()) => {
                self.selected_node_id = Some(new_id.clone());
                self.status_msg = format!("已添加节点 {}", new_id);
                self.show_add_node_dialog = false;
                self.add_node_form = AddNodeForm::default();
            }
            Err(err) => {
                self.add_node_form.error = Some(err);
            }
        }
    }

    fn delete_node(&mut self, node_id: &str) {
        let Some(data) = &self.project_data else {
            return;
        };

        let Some(task_name) = data
            .tasks
            .iter()
            .find(|task| task.id == node_id)
            .map(|task| task.name.clone())
        else {
            return;
        };

        let mut next_data = data.clone();
        next_data.tasks.retain(|task| task.id != node_id);
        next_data
            .dependencies
            .retain(|dep| dep.from != node_id && dep.to != node_id);

        match self.commit_project_data(next_data, true) {
            Ok(()) => {
                if self.selected_node_id.as_deref() == Some(node_id) {
                    self.selected_node_id = None;
                }
                if self.context_node_id.as_deref() == Some(node_id) {
                    self.context_node_id = None;
                }
                self.status_msg = format!("已删除节点 {} {}", node_id, task_name);
            }
            Err(err) => {
                self.status_msg = format!("删除失败：{}", err);
            }
        }
    }

    fn send_message(&mut self) {
        let input = self.user_input.trim().to_string();
        if input.is_empty() || self.mode == RunningMode::Running {
            return;
        }

        self.chat_history.push(ChatMessage {
            is_user: true,
            content: input.clone(),
        });

        self.user_input.clear();
        self.mode = RunningMode::Running;
        self.status_msg = String::from("正在分析...");

        let (tx, rx) = mpsc::channel::<LlmMessage>();
        self.result_receiver = Some(rx);

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            let (data, _result, _summary) = mock_llm::generate_mock_project(&input);
            let json = serde_json::to_string(&data).unwrap();
            let _ = tx.send(LlmMessage::Done(json));
        });
    }

    fn poll_llm_result(&mut self) {
        if let Some(rx) = &self.result_receiver {
            match rx.try_recv() {
                Ok(LlmMessage::Done(result)) => {
                    if let Ok(data) = serde_json::from_str::<ProjectData>(&result) {
                        if let Ok(cp) = compute_critical_path_from_json(&result) {
                            if let Ok(net) = data.clone().into_net() {
                                self.set_graph_layout(compute_layout(&net, &cp), true);
                                self.project_data = Some(data);
                                self.critical_path = Some(cp);

                                let summary = format!(
                                    "分析完成\n共 {} 个任务，关键路径：{}\n预计工期：{:.1} 天",
                                    self.critical_path.as_ref().unwrap().schedules.len(),
                                    self.critical_path
                                        .as_ref()
                                        .unwrap()
                                        .critical_path
                                        .join(" → "),
                                    self.critical_path.as_ref().unwrap().project_duration,
                                );
                                self.chat_history.push(ChatMessage {
                                    is_user: false,
                                    content: summary,
                                });
                            }
                        }
                    }
                    self.mode = RunningMode::Idle;
                    self.status_msg = String::from("完成");
                    self.result_receiver = None;
                }
                Ok(LlmMessage::Error(e)) => {
                    self.status_msg = format!("错误：{}", e);
                    self.mode = RunningMode::Idle;
                    self.result_receiver = None;
                }
                _ => {}
            }
        }
    }
}

// ── Graph drawing ──────────────────────────────────────────────────

impl InputWindow {
    fn draw_aoe_graph(&mut self, ui: &mut Ui, layout: &GraphLayout) {
        let graph_is_empty = layout.nodes.is_empty();
        let world_size = Self::graph_world_size(layout);

        let mut zoom_target = None;
        let mut fit_requested = false;

        ui.horizontal(|ui| {
            if ui.button("-").clicked() {
                zoom_target = Some(self.graph_zoom / GRAPH_ZOOM_STEP);
            }

            let mut slider_zoom = self.graph_zoom;
            if ui
                .add_sized(
                    [128.0, 18.0],
                    egui::Slider::new(&mut slider_zoom, GRAPH_ZOOM_MIN..=GRAPH_ZOOM_MAX)
                        .show_value(false)
                        .fixed_decimals(2),
                )
                .changed()
            {
                zoom_target = Some(slider_zoom);
            }

            if ui.button("+").clicked() {
                zoom_target = Some(self.graph_zoom * GRAPH_ZOOM_STEP);
            }

            ui.label(format!("{:.0}%", self.graph_zoom * 100.0));

            if ui.button("1:1").clicked() {
                zoom_target = Some(1.0);
            }

            if ui.button("适配").clicked() {
                fit_requested = true;
            }

            ui.separator();
            let add_button = egui::Button::new("+ 添加节点")
                .fill(Color32::from_rgb(64, 118, 205))
                .stroke(Stroke::new(1.0, Color32::from_rgb(42, 88, 170)))
                .min_size(Vec2::new(92.0, 24.0));
            if ui.add(add_button).clicked() {
                self.open_add_node_dialog(&[], &[]);
            }

            if let Some(cp) = &self.critical_path {
                ui.separator();
                ui.label(format!("工期 {:.1} 天", cp.project_duration));
            }
        });

        ui.add_space(6.0);

        let available = ui.available_size();
        let canvas_size = Vec2::new(available.x.max(1.0), available.y.max(1.0));
        let (response, painter) = ui.allocate_painter(canvas_size, Sense::click_and_drag());
        let view_rect = response.rect;

        let transition = if fit_requested {
            self.graph_transition = None;
            None
        } else {
            self.graph_transition_frame(ui)
        };

        if self.graph_fit_pending {
            if let Some(transition) = &transition {
                let (target_zoom, target_pan) = Self::fit_graph_transform(view_rect, world_size);
                self.graph_zoom =
                    Self::lerp_f32(transition.from_zoom, target_zoom, transition.eased_progress);
                self.graph_pan =
                    Self::lerp_vec2(transition.from_pan, target_pan, transition.eased_progress);
            } else {
                self.fit_graph_to_view(view_rect, world_size);
                self.graph_fit_pending = false;
                zoom_target = None;
            }
        } else if fit_requested {
            self.fit_graph_to_view(view_rect, world_size);
            self.graph_fit_pending = false;
            zoom_target = None;
        }

        if let Some(target) = zoom_target {
            self.set_graph_zoom_around(target, view_rect.center(), view_rect);
        }

        self.handle_graph_input(ui, &response, view_rect);
        self.constrain_graph_pan(view_rect, world_size);

        if response.secondary_clicked() {
            self.context_node_id = response
                .interact_pointer_pos()
                .and_then(|pos| self.hit_test_node(view_rect, layout, pos));
            if let Some(node_id) = &self.context_node_id {
                self.selected_node_id = Some(node_id.clone());
            }
        }

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.selected_node_id = self.hit_test_node(view_rect, layout, pos);
            }
        }

        if response.hovered()
            && ui
                .input(|i| i.pointer.hover_pos())
                .and_then(|pos| self.hit_test_node(view_rect, layout, pos))
                .is_some()
        {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        let bg = if ui.visuals().dark_mode {
            Color32::from_rgb(22, 25, 29)
        } else {
            Color32::from_rgb(247, 249, 252)
        };
        painter.rect_filled(view_rect, CornerRadius::same(6), bg);
        self.draw_graph_grid(&painter, view_rect);

        if graph_is_empty {
            painter.text(
                view_rect.center(),
                Align2::CENTER_CENTER,
                "图中没有任务节点",
                FontId::proportional(14.0),
                Color32::from_rgb(130, 135, 145),
            );
        } else if let Some(transition) = &transition {
            self.draw_transitioning_graph(
                ui,
                &painter,
                view_rect,
                &transition.from,
                layout,
                transition.raw_progress,
                transition.eased_progress,
            );
        } else {
            self.draw_graph_layout(ui, &painter, view_rect, layout);
        }

        self.draw_graph_legend(&painter, view_rect);

        egui::Popup::context_menu(&response)
            .width(180.0)
            .show(|ui| self.draw_graph_context_menu(ui));

        if !response.context_menu_opened() {
            self.context_node_id = None;
        }
    }

    fn draw_graph_context_menu(&mut self, ui: &mut Ui) {
        ui.set_min_width(180.0);
        ui.set_max_width(240.0);

        if let Some(node_id) = self.context_node_id.clone() {
            let label = self.task_label(&node_id).unwrap_or_else(|| node_id.clone());
            ui.add(
                egui::Label::new(egui::RichText::new(Self::fit_text(&label, 210.0, 13.0)).strong())
                    .wrap(),
            );
            ui.separator();

            if ui.button("添加前置节点").clicked() {
                self.open_add_node_dialog(&[], &[node_id.clone()]);
                ui.close();
            }

            if ui.button("添加后续节点").clicked() {
                self.open_add_node_dialog(&[node_id.clone()], &[]);
                ui.close();
            }

            ui.separator();
            if ui.button("删除节点").clicked() {
                self.delete_node(&node_id);
                ui.close();
            }
        } else if ui.button("添加节点").clicked() {
            self.open_add_node_dialog(&[], &[]);
            ui.close();
        }
    }

    fn graph_transition_frame(&mut self, ui: &Ui) -> Option<GraphTransitionFrame> {
        let snapshot = self.graph_transition.as_ref().map(|transition| {
            (
                transition.from.clone(),
                transition.started_at.elapsed().as_secs_f32(),
                transition.from_zoom,
                transition.from_pan,
            )
        });

        let Some((from, elapsed, from_zoom, from_pan)) = snapshot else {
            return None;
        };

        let raw_progress = (elapsed / GRAPH_TRANSITION_SECONDS).clamp(0.0, 1.0);
        if raw_progress >= 1.0 {
            self.graph_transition = None;
            return None;
        }

        ui.ctx().request_repaint();
        Some(GraphTransitionFrame {
            from,
            raw_progress,
            eased_progress: Self::ease_out_cubic(raw_progress),
            from_zoom,
            from_pan,
        })
    }

    fn draw_graph_layout(
        &self,
        ui: &Ui,
        painter: &egui::Painter,
        view_rect: Rect,
        layout: &GraphLayout,
    ) {
        let rect_by_id = self.layout_screen_rect_map(view_rect, layout);
        self.draw_layout_edges(painter, layout, &rect_by_id, 1.0, None);

        for node in &layout.nodes {
            if let Some(rect) = rect_by_id.get(&node.id) {
                self.draw_graph_node(ui, painter, node, *rect, 1.0);
            }
        }
    }

    fn draw_transitioning_graph(
        &self,
        ui: &Ui,
        painter: &egui::Painter,
        view_rect: Rect,
        from_layout: &GraphLayout,
        to_layout: &GraphLayout,
        raw_progress: f32,
        eased_progress: f32,
    ) {
        let from_rects = self.layout_screen_rect_map(view_rect, from_layout);
        let to_rects = self.layout_screen_rect_map(view_rect, to_layout);
        let old_edges = Self::edge_id_set(from_layout);
        let new_edges = Self::edge_id_set(to_layout);

        let mut animated_rects = HashMap::new();
        for node in &to_layout.nodes {
            if let Some(to_rect) = to_rects.get(&node.id) {
                let rect = if let Some(from_rect) = from_rects.get(&node.id) {
                    Self::lerp_rect(*from_rect, *to_rect, eased_progress)
                } else {
                    let offset = Vec2::new(28.0 * (1.0 - eased_progress), 0.0);
                    let scale = 0.68 + 0.32 * Self::ease_out_back(raw_progress).min(1.0);
                    Self::scale_rect_from_center(
                        Rect::from_center_size(to_rect.center() - offset, to_rect.size()),
                        scale,
                    )
                };
                animated_rects.insert(node.id.clone(), rect);
            }
        }

        for &(from_idx, to_idx) in &from_layout.edges {
            let from_node = &from_layout.nodes[from_idx];
            let to_node = &from_layout.nodes[to_idx];
            let edge_key = (from_node.id.clone(), to_node.id.clone());
            if new_edges.contains(&edge_key) {
                continue;
            }

            if let (Some(from_rect), Some(to_rect)) =
                (from_rects.get(&from_node.id), from_rects.get(&to_node.id))
            {
                self.draw_graph_edge(
                    painter,
                    from_node,
                    to_node,
                    *from_rect,
                    *to_rect,
                    1.0 - eased_progress,
                );
            }
        }

        self.draw_layout_edges(
            painter,
            to_layout,
            &animated_rects,
            1.0,
            Some((&old_edges, eased_progress)),
        );

        for node in &from_layout.nodes {
            if to_rects.contains_key(&node.id) {
                continue;
            }

            if let Some(rect) = from_rects.get(&node.id) {
                let fade_rect = Self::scale_rect_from_center(*rect, 1.0 - 0.16 * eased_progress);
                self.draw_graph_node(ui, painter, node, fade_rect, 1.0 - eased_progress);
            }
        }

        for node in &to_layout.nodes {
            if let Some(rect) = animated_rects.get(&node.id) {
                let alpha = if from_rects.contains_key(&node.id) {
                    1.0
                } else {
                    eased_progress
                };
                self.draw_graph_node(ui, painter, node, *rect, alpha);
            }
        }
    }

    fn draw_layout_edges(
        &self,
        painter: &egui::Painter,
        layout: &GraphLayout,
        rect_by_id: &HashMap<String, Rect>,
        base_alpha: f32,
        new_edge_alpha: Option<(&HashSet<(String, String)>, f32)>,
    ) {
        for &(from_idx, to_idx) in &layout.edges {
            let from_node = &layout.nodes[from_idx];
            let to_node = &layout.nodes[to_idx];

            let Some(from_rect) = rect_by_id.get(&from_node.id) else {
                continue;
            };
            let Some(to_rect) = rect_by_id.get(&to_node.id) else {
                continue;
            };

            let mut alpha = base_alpha;
            if let Some((known_edges, new_alpha)) = new_edge_alpha {
                let edge_key = (from_node.id.clone(), to_node.id.clone());
                if !known_edges.contains(&edge_key) {
                    alpha *= new_alpha;
                }
            }

            self.draw_graph_edge(painter, from_node, to_node, *from_rect, *to_rect, alpha);
        }
    }

    fn draw_graph_edge(
        &self,
        painter: &egui::Painter,
        from_node: &NodeLayout,
        to_node: &NodeLayout,
        from_rect: Rect,
        to_rect: Rect,
        alpha: f32,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.02 {
            return;
        }

        let start = from_rect.right_center();
        let end = to_rect.left_center();
        let is_critical = from_node.is_critical && to_node.is_critical;
        let color = Self::fade_color(
            if is_critical {
                EDGE_CRITICAL
            } else {
                EDGE_NORMAL
            },
            alpha,
        );
        let width = if is_critical { 2.5 } else { 1.5 };
        let stroke_width = (width * self.graph_zoom).clamp(1.0, 4.5);

        painter.line_segment([start, end], Stroke::new(stroke_width, color));

        let edge = end - start;
        if edge.length() > 1.0 {
            let dir = edge.normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let sz = (8.0 * self.graph_zoom).clamp(5.0, 13.0);
            let tip = end;
            let a = tip - dir * sz + perp * sz * 0.36;
            let b = tip - dir * sz - perp * sz * 0.36;
            painter.line_segment([tip, a], Stroke::new(stroke_width, color));
            painter.line_segment([tip, b], Stroke::new(stroke_width, color));
        }
    }

    fn draw_graph_node(
        &self,
        ui: &Ui,
        painter: &egui::Painter,
        node: &NodeLayout,
        rect: Rect,
        alpha: f32,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.02 {
            return;
        }

        let (fill, border, text_c) = if node.is_critical {
            (CRITICAL_FILL, CRITICAL_BORDER, CRITICAL_TEXT)
        } else {
            (NORMAL_FILL, NORMAL_BORDER, NORMAL_TEXT)
        };

        let radius = CornerRadius::same((8.0 * self.graph_zoom).clamp(4.0, 10.0) as u8);
        let shadow = Vec2::new(2.0, 3.0) * self.graph_zoom.clamp(0.7, 1.5);
        let shadow_alpha = ((if ui.visuals().dark_mode { 65.0 } else { 24.0 }) * alpha) as u8;

        painter.rect_filled(
            rect.translate(shadow),
            radius,
            Color32::from_black_alpha(shadow_alpha),
        );

        painter.rect_filled(rect, radius, Self::fade_color(fill, alpha));
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(
                (2.0 * self.graph_zoom).clamp(1.0, 3.0),
                Self::fade_color(border, alpha),
            ),
            StrokeKind::Middle,
        );

        if self.selected_node_id.as_deref() == Some(node.id.as_str()) {
            painter.rect_stroke(
                rect.expand((4.0 * self.graph_zoom).clamp(2.0, 6.0)),
                CornerRadius::same((10.0 * self.graph_zoom).clamp(5.0, 12.0) as u8),
                Stroke::new(
                    (2.0 * self.graph_zoom).clamp(1.5, 3.5),
                    Self::fade_color(Color32::from_rgb(255, 180, 70), alpha),
                ),
                StrokeKind::Middle,
            );
        }

        let title_size = (13.0 * self.graph_zoom).clamp(8.5, 15.0);
        let detail_size = (11.0 * self.graph_zoom).clamp(8.0, 13.0);
        let name = Self::fit_text(&node.name, (rect.width() - 12.0).max(4.0), title_size);
        let compact_node = rect.height() < 34.0;

        painter.text(
            if compact_node {
                rect.center()
            } else {
                rect.center() - Vec2::new(0.0, 8.5 * self.graph_zoom.clamp(0.6, 1.3))
            },
            Align2::CENTER_CENTER,
            name,
            FontId::proportional(title_size),
            Self::fade_color(text_c, alpha),
        );

        if !compact_node {
            painter.text(
                rect.center() + Vec2::new(0.0, 11.0 * self.graph_zoom.clamp(0.6, 1.3)),
                Align2::CENTER_CENTER,
                format!("{:.1} 天", node.duration),
                FontId::proportional(detail_size),
                Self::fade_color(DURATION_TEXT, alpha),
            );
        }
    }

    fn layout_screen_rect_map(
        &self,
        view_rect: Rect,
        layout: &GraphLayout,
    ) -> HashMap<String, Rect> {
        let layer_sizes = self.layer_sizes(layout);
        let world_size = Self::graph_world_size(layout);
        Self::node_world_rects(layout, &layer_sizes, world_size)
            .into_iter()
            .zip(layout.nodes.iter())
            .map(|(rect, node)| (node.id.clone(), self.world_rect_to_screen(view_rect, rect)))
            .collect()
    }

    fn hit_test_node(&self, view_rect: Rect, layout: &GraphLayout, pos: Pos2) -> Option<String> {
        let rect_by_id = self.layout_screen_rect_map(view_rect, layout);
        layout
            .nodes
            .iter()
            .rev()
            .find(|node| {
                rect_by_id
                    .get(&node.id)
                    .is_some_and(|rect| rect.contains(pos))
            })
            .map(|node| node.id.clone())
    }

    fn edge_id_set(layout: &GraphLayout) -> HashSet<(String, String)> {
        layout
            .edges
            .iter()
            .map(|&(from_idx, to_idx)| {
                (
                    layout.nodes[from_idx].id.clone(),
                    layout.nodes[to_idx].id.clone(),
                )
            })
            .collect()
    }

    fn graph_world_size(layout: &GraphLayout) -> Vec2 {
        let layer_count = layout.layer_count.max(1);
        let layer_span = layer_count.saturating_sub(1) as f32 * H_GAP;
        let row_count = layout.max_nodes_in_layer.max(1);
        let row_span = row_count.saturating_sub(1) as f32 * V_GAP;

        Vec2::new(
            MARGIN * 2.0 + NODE_W + layer_span,
            MARGIN * 2.0 + NODE_H + row_span,
        )
    }

    fn node_world_rects(
        layout: &GraphLayout,
        layer_sizes: &[usize],
        world_size: Vec2,
    ) -> Vec<Rect> {
        layout
            .nodes
            .iter()
            .map(|node| {
                let gx = MARGIN + node.layer as f32 * H_GAP;
                let nodes_in_layer = layer_sizes.get(node.layer).copied().unwrap_or(1).max(1);
                let layer_h = NODE_H + nodes_in_layer.saturating_sub(1) as f32 * V_GAP;
                let start_gy = (world_size.y - layer_h) / 2.0;
                let gy = start_gy + node.index_in_layer as f32 * V_GAP;

                Rect::from_min_size(Pos2::new(gx, gy), Vec2::new(NODE_W, NODE_H))
            })
            .collect()
    }

    fn fit_graph_to_view(&mut self, view_rect: Rect, world_size: Vec2) {
        let (zoom, pan) = Self::fit_graph_transform(view_rect, world_size);
        self.graph_zoom = zoom;
        self.graph_pan = pan;
    }

    fn fit_graph_transform(view_rect: Rect, world_size: Vec2) -> (f32, Vec2) {
        let view_w = (view_rect.width() - VIEW_PADDING * 2.0).max(1.0);
        let view_h = (view_rect.height() - VIEW_PADDING * 2.0).max(1.0);
        let zoom = (view_w / world_size.x)
            .min(view_h / world_size.y)
            .clamp(GRAPH_ZOOM_MIN, GRAPH_ZOOM_MAX);

        let pan = view_rect.center() - view_rect.min - world_size * zoom * 0.5;
        (zoom, pan)
    }

    fn handle_graph_input(&mut self, ui: &Ui, response: &egui::Response, view_rect: Rect) {
        let is_dragging = response.dragged_by(egui::PointerButton::Primary)
            || response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle);

        if is_dragging {
            self.graph_pan += response.drag_delta();
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        } else if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }

        if response.hovered() {
            let zoom_factor = ui.input(|i| {
                let pinch = i.zoom_delta();
                if (pinch - 1.0).abs() > f32::EPSILON {
                    pinch
                } else {
                    (i.smooth_scroll_delta.y * 0.0018).exp()
                }
            });

            if (zoom_factor - 1.0).abs() > 0.001 {
                let anchor = ui
                    .input(|i| i.pointer.hover_pos())
                    .unwrap_or_else(|| view_rect.center());
                self.set_graph_zoom_around(self.graph_zoom * zoom_factor, anchor, view_rect);
            }
        }
    }

    fn set_graph_zoom_around(&mut self, target_zoom: f32, anchor: Pos2, view_rect: Rect) {
        let old_zoom = self.graph_zoom;
        let new_zoom = target_zoom.clamp(GRAPH_ZOOM_MIN, GRAPH_ZOOM_MAX);

        if (new_zoom - old_zoom).abs() < 0.0001 {
            return;
        }

        let anchor_in_view = anchor - view_rect.min;
        let world_under_anchor = (anchor_in_view - self.graph_pan) / old_zoom;
        self.graph_zoom = new_zoom;
        self.graph_pan = anchor_in_view - world_under_anchor * new_zoom;
    }

    fn constrain_graph_pan(&mut self, view_rect: Rect, world_size: Vec2) {
        let content = world_size * self.graph_zoom;
        let view = view_rect.size();
        let overscroll = 80.0;

        self.graph_pan.x = if content.x <= view.x {
            (view.x - content.x) * 0.5
        } else {
            self.graph_pan
                .x
                .clamp(view.x - content.x - overscroll, overscroll)
        };

        self.graph_pan.y = if content.y <= view.y {
            (view.y - content.y) * 0.5
        } else {
            self.graph_pan
                .y
                .clamp(view.y - content.y - overscroll, overscroll)
        };
    }

    fn world_rect_to_screen(&self, view_rect: Rect, rect: Rect) -> Rect {
        Rect::from_min_max(
            self.world_to_screen(view_rect, rect.min),
            self.world_to_screen(view_rect, rect.max),
        )
    }

    fn world_to_screen(&self, view_rect: Rect, pos: Pos2) -> Pos2 {
        view_rect.min + self.graph_pan + pos.to_vec2() * self.graph_zoom
    }

    fn lerp_f32(from: f32, to: f32, t: f32) -> f32 {
        from + (to - from) * t
    }

    fn lerp_vec2(from: Vec2, to: Vec2, t: f32) -> Vec2 {
        from + (to - from) * t
    }

    fn lerp_pos(from: Pos2, to: Pos2, t: f32) -> Pos2 {
        from + (to - from) * t
    }

    fn lerp_rect(from: Rect, to: Rect, t: f32) -> Rect {
        Rect::from_min_max(
            Self::lerp_pos(from.min, to.min, t),
            Self::lerp_pos(from.max, to.max, t),
        )
    }

    fn scale_rect_from_center(rect: Rect, scale: f32) -> Rect {
        Rect::from_center_size(rect.center(), rect.size() * scale)
    }

    fn ease_out_cubic(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(3)
    }

    fn ease_out_back(t: f32) -> f32 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
    }

    fn fade_color(color: Color32, alpha: f32) -> Color32 {
        let alpha = alpha.clamp(0.0, 1.0);
        Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            (color.a() as f32 * alpha).round() as u8,
        )
    }

    fn draw_graph_grid(&self, painter: &egui::Painter, view_rect: Rect) {
        let spacing = (80.0 * self.graph_zoom).clamp(32.0, 120.0);
        let color = Color32::from_gray(if painter.ctx().global_style().visuals.dark_mode {
            39
        } else {
            225
        });
        let stroke = Stroke::new(1.0, color);

        let mut x = view_rect.left() + self.graph_pan.x.rem_euclid(spacing);
        if x > view_rect.left() {
            x -= spacing;
        }
        while x <= view_rect.right() {
            painter.line_segment(
                [
                    Pos2::new(x, view_rect.top()),
                    Pos2::new(x, view_rect.bottom()),
                ],
                stroke,
            );
            x += spacing;
        }

        let mut y = view_rect.top() + self.graph_pan.y.rem_euclid(spacing);
        if y > view_rect.top() {
            y -= spacing;
        }
        while y <= view_rect.bottom() {
            painter.line_segment(
                [
                    Pos2::new(view_rect.left(), y),
                    Pos2::new(view_rect.right(), y),
                ],
                stroke,
            );
            y += spacing;
        }
    }

    fn draw_graph_legend(&self, painter: &egui::Painter, view_rect: Rect) {
        let rect = Rect::from_min_size(
            view_rect.left_top() + Vec2::new(12.0, 12.0),
            Vec2::new(232.0, 34.0),
        );
        let dark_mode = painter.ctx().global_style().visuals.dark_mode;
        let bg = if dark_mode {
            Color32::from_black_alpha(150)
        } else {
            Color32::from_white_alpha(210)
        };

        painter.rect_filled(rect, CornerRadius::same(6), bg);

        let lx = rect.left() + 10.0;
        let cy = rect.center().y;

        painter.rect_filled(
            Rect::from_center_size(Pos2::new(lx + 7.0, cy), Vec2::new(14.0, 14.0)),
            CornerRadius::same(3),
            CRITICAL_FILL,
        );
        painter.rect_stroke(
            Rect::from_center_size(Pos2::new(lx + 7.0, cy), Vec2::new(14.0, 14.0)),
            CornerRadius::same(3),
            Stroke::new(1.5, CRITICAL_BORDER),
            StrokeKind::Middle,
        );
        painter.text(
            Pos2::new(lx + 20.0, cy),
            Align2::LEFT_CENTER,
            "关键路径",
            FontId::proportional(12.0),
            CRITICAL_TEXT,
        );

        let nx = lx + 98.0;
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(nx + 7.0, cy), Vec2::new(14.0, 14.0)),
            CornerRadius::same(3),
            NORMAL_FILL,
        );
        painter.rect_stroke(
            Rect::from_center_size(Pos2::new(nx + 7.0, cy), Vec2::new(14.0, 14.0)),
            CornerRadius::same(3),
            Stroke::new(1.5, NORMAL_BORDER),
            StrokeKind::Middle,
        );
        painter.text(
            Pos2::new(nx + 20.0, cy),
            Align2::LEFT_CENTER,
            "非关键任务",
            FontId::proportional(12.0),
            NORMAL_TEXT,
        );
    }

    fn fit_text(text: &str, max_width: f32, font_size: f32) -> String {
        let char_count = text.chars().count();
        let max_chars = (max_width / (font_size * 0.85)).floor().max(1.0) as usize;

        if char_count <= max_chars {
            return text.to_owned();
        }

        if max_chars <= 3 {
            return text.chars().take(max_chars).collect();
        }

        let mut clipped: String = text.chars().take(max_chars - 3).collect();
        clipped.push_str("...");
        clipped
    }

    fn layer_sizes(&self, layout: &GraphLayout) -> Vec<usize> {
        let mut sizes = vec![0usize; layout.layer_count.max(1)];
        for node in &layout.nodes {
            if node.layer < sizes.len() {
                sizes[node.layer] = sizes[node.layer].max(node.index_in_layer + 1);
            }
        }
        sizes
    }

    fn draw_add_node_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_add_node_dialog {
            return;
        }

        let task_options: Vec<(String, String)> = self
            .project_data
            .as_ref()
            .map(|data| {
                data.tasks
                    .iter()
                    .map(|task| (task.id.clone(), format!("{} {}", task.id, task.name)))
                    .collect()
            })
            .unwrap_or_default();

        let mut open = self.show_add_node_dialog;
        let mut add_clicked = false;
        let mut cancel_clicked = false;

        egui::Window::new("添加节点")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.label("任务名");
                ui.add(
                    egui::TextEdit::singleline(&mut self.add_node_form.name)
                        .desired_width(f32::INFINITY),
                );

                ui.add_space(8.0);
                ui.label("三点估算工期");
                ui.horizontal(|ui| {
                    ui.label("乐观");
                    ui.add(
                        egui::DragValue::new(&mut self.add_node_form.duration_optimistic)
                            .speed(0.1)
                            .range(0.1..=999.0)
                            .suffix(" 天"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("最可能");
                    ui.add(
                        egui::DragValue::new(&mut self.add_node_form.duration_normal)
                            .speed(0.1)
                            .range(0.1..=999.0)
                            .suffix(" 天"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("悲观");
                    ui.add(
                        egui::DragValue::new(&mut self.add_node_form.duration_pessimistic)
                            .speed(0.1)
                            .range(0.1..=999.0)
                            .suffix(" 天"),
                    );
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("依赖关系");
                    ui.weak("可多选");
                });
                ui.columns(2, |columns| {
                    columns[0].push_id("predecessor_tasks", |ui| {
                        ui.label("前置任务");
                        ui.weak("选中任务 -> 新任务");
                        egui::ScrollArea::vertical()
                            .id_salt("predecessor_tasks_scroll")
                            .max_height(150.0)
                            .show(ui, |ui| {
                                if task_options.is_empty() {
                                    ui.weak("暂无可选任务");
                                }

                                for (task_id, label) in &task_options {
                                    let mut selected =
                                        self.add_node_form.predecessor_ids.contains(task_id);
                                    if ui.checkbox(&mut selected, label).changed() {
                                        if selected {
                                            self.add_node_form
                                                .predecessor_ids
                                                .insert(task_id.clone());
                                            self.add_node_form.successor_ids.remove(task_id);
                                        } else {
                                            self.add_node_form.predecessor_ids.remove(task_id);
                                        }
                                    }
                                }
                            });
                    });

                    columns[1].push_id("successor_tasks", |ui| {
                        ui.label("后续任务");
                        ui.weak("新任务 -> 选中任务");
                        egui::ScrollArea::vertical()
                            .id_salt("successor_tasks_scroll")
                            .max_height(150.0)
                            .show(ui, |ui| {
                                if task_options.is_empty() {
                                    ui.weak("暂无可选任务");
                                }

                                for (task_id, label) in &task_options {
                                    let mut selected =
                                        self.add_node_form.successor_ids.contains(task_id);
                                    if ui.checkbox(&mut selected, label).changed() {
                                        if selected {
                                            self.add_node_form
                                                .successor_ids
                                                .insert(task_id.clone());
                                            self.add_node_form.predecessor_ids.remove(task_id);
                                        } else {
                                            self.add_node_form.successor_ids.remove(task_id);
                                        }
                                    }
                                }
                            });
                    });
                });

                if let Some(error) = &self.add_node_form.error {
                    ui.add_space(8.0);
                    ui.colored_label(Color32::from_rgb(190, 55, 45), error);
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    add_clicked = ui.button("添加").clicked();
                    cancel_clicked = ui.button("取消").clicked();
                });
            });

        if cancel_clicked {
            open = false;
            self.add_node_form.error = None;
        }

        self.show_add_node_dialog = open;

        if add_clicked {
            self.add_manual_node();
        }
    }

    fn draw_node_detail_panel(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("节点信息");
            if self.selected_node_id.is_some() && ui.button("取消选择").clicked() {
                self.selected_node_id = None;
            }
        });
        ui.separator();

        let Some(selected_id) = self.selected_node_id.as_deref() else {
            ui.weak("点击图中的节点查看详细信息");
            return;
        };

        let Some(data) = &self.project_data else {
            ui.weak("当前没有项目数据");
            return;
        };

        let Some(task) = data.tasks.iter().find(|task| task.id == selected_id) else {
            ui.weak("选中的节点已不存在");
            return;
        };

        let schedule = self
            .critical_path
            .as_ref()
            .and_then(|result| result.schedules.iter().find(|item| item.task_id == task.id));

        ui.label(format!("ID: {}", task.id));
        ui.add(egui::Label::new(format!("名称: {}", task.name)).wrap());
        ui.label(format!(
            "三点估算: {:.1} / {:.1} / {:.1} 天",
            task.duration_optimistic, task.duration_normal, task.duration_pessimistic
        ));
        ui.label(format!("PERT 工期: {:.1} 天", task.pert()));

        if let Some(schedule) = schedule {
            ui.add_space(6.0);
            if schedule.is_critical {
                ui.colored_label(CRITICAL_TEXT, "关键任务");
            } else {
                ui.colored_label(NORMAL_TEXT, "非关键任务");
            }
            ui.label(format!(
                "最早: {:.1} - {:.1}",
                schedule.earliest_start, schedule.earliest_finish
            ));
            ui.label(format!(
                "最晚: {:.1} - {:.1}",
                schedule.latest_start, schedule.latest_finish
            ));
            ui.label(format!("总时差: {:.1} 天", schedule.slack));
        }

        let predecessors = self.task_refs_for(selected_id, true);
        let successors = self.task_refs_for(selected_id, false);

        ui.add_space(6.0);
        ui.add(egui::Label::new(format!("前置: {}", predecessors)).wrap());
        ui.add(egui::Label::new(format!("后续: {}", successors)).wrap());
    }

    fn task_refs_for(&self, task_id: &str, incoming: bool) -> String {
        let Some(data) = &self.project_data else {
            return String::from("无");
        };

        let refs: Vec<String> = data
            .dependencies
            .iter()
            .filter_map(|dep| {
                let related_id = if incoming && dep.to == task_id {
                    Some(dep.from.as_str())
                } else if !incoming && dep.from == task_id {
                    Some(dep.to.as_str())
                } else {
                    None
                }?;

                data.tasks
                    .iter()
                    .find(|task| task.id == related_id)
                    .map(|task| format!("{} {}", task.id, task.name))
            })
            .collect();

        if refs.is_empty() {
            String::from("无")
        } else {
            refs.join(", ")
        }
    }

    fn task_label(&self, task_id: &str) -> Option<String> {
        self.project_data.as_ref().and_then(|data| {
            data.tasks
                .iter()
                .find(|task| task.id == task_id)
                .map(|task| format!("{} {}", task.id, task.name))
        })
    }

    fn draw_chat_panel(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("对话记录");
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.chat_history.is_empty() {
                    ui.weak("输入项目描述后，分析结果将显示在这里...");
                }
                for msg in &self.chat_history {
                    ui.add_space(4.0);
                    if msg.is_user {
                        ui.colored_label(Color32::from_rgb(60, 100, 180), "你");
                    } else {
                        ui.colored_label(Color32::from_rgb(180, 80, 60), "助手");
                    }
                    ui.add(egui::Label::new(&msg.content).wrap());
                    ui.add_space(2.0);
                    ui.separator();
                }
            });
    }
}

// ── eframe::App ────────────────────────────────────────────────────

impl eframe::App for InputWindow {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        self.poll_llm_result();

        if self.mode == RunningMode::Running {
            ui.ctx().request_repaint();
        }

        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("CriticalFlow");
                ui.separator();
                ui.label(&self.status_msg);
                if self.mode == RunningMode::Running {
                    ui.spinner();
                }
            });
        });

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let button_width = 64.0;
                let spacing = ui.spacing().item_spacing.x;
                let input_width = (ui.available_width() - button_width - spacing).max(180.0);
                let input_field = ui.add(
                    egui::TextEdit::singleline(&mut self.user_input)
                        .hint_text("描述你的项目任务")
                        .desired_width(input_width),
                );

                if input_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.send_message();
                }

                let btn = ui.add_enabled(
                    self.mode == RunningMode::Idle,
                    egui::Button::new("发送").min_size(Vec2::new(button_width, 24.0)),
                );

                if btn.clicked() {
                    self.send_message();
                }
            });
            ui.add_space(4.0);
        });

        egui::Panel::right("chat_panel")
            .min_size(220.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_node_detail_panel(ui);
                ui.add_space(10.0);
                self.draw_chat_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(layout) = self.graph_layout.clone() {
                self.draw_aoe_graph(ui, &layout);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("输入任务描述后，AOE图将显示在这里");
                });
            }
        });

        self.draw_add_node_dialog(ui.ctx());
    }
}
