#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, io::Cursor, sync::Arc, thread, time::Duration};

use anyhow::{Context, Result, ensure};
use battery::{Manager, State};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontRef, PxScale, Font, ScaleFont};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder, App,
};
use tokio::sync::{mpsc, Mutex};

/// 电池图标生成器类
pub struct BatteryIconGenerator {
    font: FontRef<'static>,
    cache: Mutex<HashMap<(u32, bool), Image<'static>>>,
}

impl BatteryIconGenerator {
    pub fn new() -> Result<Self> {
        let font = FontRef::try_from_slice(include_bytes!("../assets/ComicMono.ttf"))
            .context("failed to load font")?;
        Ok(Self {
            font,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// 二分法寻找合适的宽度
    fn find_scale_for_width(&self, text: &str, target_width: f32) -> PxScale {
        let mut low = 1.0;
        let mut high = 200.0;
        let tolerance = 0.1;

        while high - low > tolerance {
            let mid = (low + high) / 2.0;
            let scaled_font = self.font.as_scaled(PxScale::from(mid));
            let width: f32 = text.chars().map(|c| scaled_font.h_advance(scaled_font.glyph_id(c))).sum();

            if width < target_width {
                low = mid;
            } else {
                high = mid;
            }
        }

        PxScale::from((low + high) / 2.0)
    }

    /// 计算字符坐标，使其水平垂直居中
    fn compute_pos_from_scale(&self, text: &str, scale: PxScale) -> (i32, i32) {
        let scaled_font = self.font.as_scaled(scale);
        let width: f32 = text.chars().map(|c| scaled_font.h_advance(scaled_font.glyph_id(c))).sum();
        let char_height = scaled_font.ascent();
        let x = (64.0 - width) / 2.0;
        let y = (64.0 - char_height) / 2.0;
        (x as i32, y as i32)
    }

    /// 生成电池电量图标（64x64，白底黑字）
    pub async fn generate_icon(&self, percentage: u32, charging: bool) -> Result<Image<'static>> {
        ensure!((0..=100).contains(&percentage), "Battery percentage must be between 0 and 100");

        // 尝试从缓存获取
        let key = (percentage, charging);
        if let Some(cached_icon) = self.cache.lock().await.get(&key) {
            return Ok(cached_icon.clone());
        }

        let text = if charging {
            format!("{percentage}*")
        } else {
            format!("{percentage}")
        };

        const SIZE: u32 = 64;
        let mut img = RgbaImage::new(SIZE, SIZE);
        let scale = self.find_scale_for_width(&text, SIZE as f32);
        let (x, y) = self.compute_pos_from_scale(&text, scale);

        draw_text_mut(&mut img, Rgba([0, 0, 0, 255]), x, y, scale, &self.font, &text);

        let mut icon_data = Cursor::new(Vec::new());
        img.write_to(&mut icon_data, image::ImageFormat::Ico)
            .context("failed to encode icon to ICO")?;

        let icon_image = Image::from_bytes(&icon_data.into_inner())
            .context("failed to create Tauri image")?
            .to_owned();

        self.cache.lock().await.insert(key, icon_image.clone());

        Ok(icon_image)
    }
}

/// 在独立线程中定期读取电池电量并发送消息
pub fn spawn_battery_monitor(tx: mpsc::Sender<(u32, State)>) {
    thread::spawn(move || {
        let manager = Manager::new().expect("Failed to initialize battery manager");

        loop {
            if let Ok(batteries) = manager.batteries() {
                for battery in batteries.flatten() {
                    let percentage = (battery.state_of_charge().value * 100.0).round() as u32;
                    let state = battery.state();

                    if tx.blocking_send((percentage, state)).is_err() {
                        eprintln!("Receiver dropped, exiting battery monitor thread");
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
}

/// 初始化托盘图标和菜单
fn init_tray(app: &mut App) -> Result<(Arc<Mutex<tauri::tray::TrayIcon>>, mpsc::Receiver<(u32, State)>)> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    let tray_icon = TrayIconBuilder::new()
        .menu(&menu)
        .build(app)?;
    let tray = Arc::new(Mutex::new(tray_icon));

    let (tx, rx) = mpsc::channel(1);
    spawn_battery_monitor(tx);

    Ok((tray, rx))
}

/// 启动异步任务监听电池更新并修改托盘图标
fn spawn_tray_updater(tray: Arc<Mutex<tauri::tray::TrayIcon>>, mut rx: mpsc::Receiver<(u32, State)>) {
    tauri::async_runtime::spawn(async move {
        let icon_generator = BatteryIconGenerator::new().unwrap();
        while let Some((percentage, state)) = rx.recv().await {
            if let Ok(icon) = icon_generator.generate_icon(percentage, state == State::Charging).await {
                let tooltip = match state {
                    State::Charging => format!("Charging: {}%", percentage),
                    State::Discharging => format!("Discharging: {}%", percentage),
                    State::Full => format!("Full"),
                    _ => format!("Unhandled state: {}%", percentage)
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
