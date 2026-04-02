use std::sync::Arc;
use eframe::{egui,Frame};
use std::time::{Duration,Instant};
use egui::Context;

#[derive(PartialEq,Clone,Copy)]
enum RunningMode {
    Idle,
    Running,
}

pub struct InputWindow {
    user_input: String,
    graph_output: String,
    mode: RunningMode
}

impl Default for InputWindow {
    fn default() -> Self {
        InputWindow {
            user_input: String::from(""),
            graph_output: String::from(""),
            mode: RunningMode::Idle
        }
    }
}

impl InputWindow {
    pub fn new(cc:&eframe::CreationContext<'_>) -> Self {
        Self::set_font_custom(&cc.egui_ctx);
        Self::default()
    }

    fn set_font_custom(ctx:&egui::Context) {
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "my_chinese_font".to_owned(),
            Arc::from(egui::FontData::from_static(include_bytes!("../../../assets/SourceHanSansCN-Medium.otf")))
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0,"my_chinese_font".to_owned())

        ctx.set_fonts(fonts);
    }
}

impl eframe::App for InputWindow {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {

    }
}
