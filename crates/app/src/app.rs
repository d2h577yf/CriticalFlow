use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use eframe::{egui, Frame};
use egui::{Context, Ui};

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

pub struct InputWindow {
    user_input: String,
    chat_history: Vec<ChatMessage>,

    graph_output: String,

    mode: RunningMode,
    status_msg: String,

    llm_sender: Option<Sender<String>>,
    result_receiver: Option<Receiver<LlmMessage>>,
}

impl Default for InputWindow {
    fn default() -> Self {
        InputWindow {
            user_input: String::new(),
            chat_history: Vec::new(),
            graph_output: String::from("输入任务描述后，AOE图将显示在这里"),
            mode: RunningMode::Idle,
            status_msg: String::from("就绪"),
            llm_sender: None,
            result_receiver: None,
        }
    }
}

impl InputWindow {
    pub fn new(cc:&eframe::CreationContext<'_>) -> Self {
        Self::set_font_custom(&cc.egui_ctx);
        Self::default()
    }

    fn set_font_custom(ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "my_chinese_font".to_owned(),
            Arc::from(egui::FontData::from_static(
                include_bytes!("../../../assets/SourceHanSansCN-Medium.otf")
            ))
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "my_chinese_font".to_owned());
        ctx.set_fonts(fonts);
    }

    fn send_message(&mut self) {
        let input = self.user_input.trim().to_string();
        if input.is_empty() || self.mode == RunningMode::Running {
            return;
        }

        self.chat_history.push(ChatMessage {
            is_user:true,
            content: input.clone(),
        });

        self.user_input.clear();
        self.mode = RunningMode::Running;
        self.status_msg = String::from("正在分析...");

        let (tx,rx) = mpsc::channel::<LlmMessage>();
        self.result_receiver = Some(rx);

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = tx.send(LlmMessage::Done(
                format!("已收到：{}\n\n[这里将显示AOE图的文本表示]", input)
            ));
        });

    }
    fn poll_llm_result(&mut self) {
        if let Some(rx) = &self.result_receiver {
            match rx.try_recv() {
                Ok(LlmMessage::Done(result)) => {
                    self.chat_history.push(ChatMessage{
                        is_user: false,
                        content: result.clone()
                    });
                    self.graph_output = result;
                    self.mode = RunningMode::Idle;
                    self.status_msg = String::from("完成");
                    self.result_receiver = None;
                }
                Ok(LlmMessage::Error(e)) => {
                    self.status_msg = format!("错误：{}",e);
                    self.mode = RunningMode::Idle;
                    self.result_receiver = None;
                }
                _ => {}
            }
        }
    }
}

impl eframe::App for InputWindow {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {
        todo!()
    }
}