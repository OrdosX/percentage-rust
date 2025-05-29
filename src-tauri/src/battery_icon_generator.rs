use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use anyhow::{ensure, Context, Result};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use std::io::Cursor;
use tauri::image::Image;

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
