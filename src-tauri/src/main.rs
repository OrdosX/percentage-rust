#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{io::Cursor, sync::Arc, thread, time::Duration};

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use anyhow::{ensure, Context, Result};
use battery::{Manager, State};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use tauri::{
    async_runtime,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    App,
};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};

/// 电池图标生成器类
pub struct BatteryIconGenerator {
    font: FontRef<'static>,
}

impl BatteryIconGenerator {
    const SIZE: u32 = 64;
    pub fn new() -> Result<Self> {
        let font = FontRef::try_from_slice(include_bytes!("../assets/ComicMono.ttf"))
            .context("Failed to load font")?;
        Ok(Self { font })
    }

    /// 构造字符串，充电时添加星号，充满显示笑脸
    fn build_text(&self, percentage: u32, charging: bool) -> String {
        match (charging, percentage > 97) {
            (true, true) => "^_^".to_string(),
            (true, false) => format!("{percentage}*"),
            (false, _) => format!("{percentage}"),
        }
    }

    /// 计算字符串宽高
    fn measure_text(&self, text: &str, scale: PxScale) -> (f32, f32) {
        let scaled_font = self.font.as_scaled(scale);
        let width = text.chars().map(|c| scaled_font.h_advance(scaled_font.glyph_id(c))).sum();
        let height = scaled_font.ascent();
        (width, height)
    }

    /// 计算字符串坐标，使其水平垂直居中
    fn compute_position(&self, width: f32, height: f32) -> (i32, i32) {
        let x = (BatteryIconGenerator::SIZE as f32 - width) / 2.0;
        let y = (BatteryIconGenerator::SIZE as f32 - height) / 2.0;
        (x as i32, y as i32)
    }

    /// 二分法寻找合适的宽度
    fn find_scale_for_width(&self, text: &str) -> PxScale {
        const TOLERANCE: f32 = 0.1;

        let mut low = 1.0;
        let mut high = 200.0;

        while high - low > TOLERANCE {
            let mid = (low + high) / 2.0;
            let (width, _) = self.measure_text(text, PxScale::from(mid));
            if width < BatteryIconGenerator::SIZE as f32 {
                low = mid;
            } else {
                high = mid;
            }
        }

        PxScale::from((low + high) / 2.0)
    }

    /// 绘制图标并转为Tauri Image对象
    fn render_icon(&self, x: i32, y: i32, scale: PxScale, text: &str) -> Result<Image<'static>> {
        let mut img = RgbaImage::new(BatteryIconGenerator::SIZE, BatteryIconGenerator::SIZE);
        draw_text_mut(&mut img, Rgba([0, 0, 0, 255]), x, y, scale, &self.font, &text);

        let mut icon_data = Cursor::new(Vec::new());
        img.write_to(&mut icon_data, image::ImageFormat::Ico)
            .context("Failed to encode icon to ICO")?;

        let icon_image = Image::from_bytes(&icon_data.into_inner())
            .context("Failed to create Tauri image")?
            .to_owned();

        Ok(icon_image)
    }

    /// 生成电池电量图标（64x64，白底黑字）
    pub async fn generate_icon(&self, percentage: u32, charging: bool) -> Result<Image<'static>> {
        ensure!((0..=100).contains(&percentage), "Battery percentage must be between 0 and 100");

        let text = self.build_text(percentage, charging);
        let scale = self.find_scale_for_width(&text);
        let (width, height) = self.measure_text(&text, scale);
        let (x, y) = self.compute_position(width, height);

        self.render_icon(x, y, scale, &text)
            .context("Failed to render icon")
    }
}

/// 在独立线程中定期读取电池电量并发送消息
fn spawn_battery_monitor(tx: Sender<(u32, State)>) {
    thread::spawn(move || {
        let manager = Manager::new().expect("Failed to initialize battery manager");
        let mut last_battery_info: Option<(u32, State)> = None;

        loop {
            if let Ok(batteries) = manager.batteries() {
                if let Some(battery) = batteries.flatten().next() {
                    let percentage = (battery.state_of_charge().value * 100.0).round() as u32;
                    let state = battery.state();

                    if Some((percentage, state)) != last_battery_info {
                        last_battery_info = Some((percentage, state));

                        tx.blocking_send((percentage, state))
                            .expect("Failed to send battery info");
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
}

/// 初始化托盘图标和菜单
fn init_tray(app: &mut App) -> Result<(Arc<Mutex<tauri::tray::TrayIcon>>, Receiver<(u32, State)>)> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    let tray_icon = TrayIconBuilder::new().menu(&menu).build(app)?;
    let tray = Arc::new(Mutex::new(tray_icon));

    let (tx, rx) = channel(1);
    spawn_battery_monitor(tx);

    Ok((tray, rx))
}

/// 启动异步任务监听电池更新并修改托盘图标
fn spawn_tray_updater(tray: Arc<Mutex<TrayIcon>>, mut rx: Receiver<(u32, State)>) {
    async_runtime::spawn(async move {
        let icon_generator = BatteryIconGenerator::new().unwrap();
        while let Some((percentage, state)) = rx.recv().await {
            if let Ok(icon) = icon_generator.generate_icon(percentage, state == State::Charging).await {
                let tooltip = match state {
                    State::Charging => format!("Charging: {}%", percentage),
                    State::Discharging => format!("Discharging: {}%", percentage),
                    State::Full => format!("Full"),
                    _ => format!("Unhandled state: {}%", percentage),
                };

                let tray = tray.lock().await;
                if let Err(e) = tray.set_icon(Some(icon)) {
                    eprintln!("Failed to update tray icon: {}", e);
                }
                if let Err(e) = tray.set_tooltip(Some(&tooltip)) {
                    eprintln!("Failed to update tray tooltip: {}", e);
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let (tray, rx) = init_tray(app)?;
            spawn_tray_updater(tray, rx);
            Ok(())
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                println!("User clicked quit");
                app.exit(0);
            }
            other => {
                println!("Unhandled menu item: {:?}", other);
            }
        })
        .run(tauri::generate_context!())
        .expect("Error running Tauri app");
}
