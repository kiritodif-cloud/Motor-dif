use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};

#[derive(Debug, Clone)]
pub enum UiElement {
    Panel {
        id: String,
        anchor: String,
        offset_x: f32,
        offset_y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    },
    Bar {
        id: String,
        anchor: String,
        offset_x: f32,
        offset_y: f32,
        width: f32,
        height: f32,
        progress: f32,
        label: String,
        color: [f32; 3],
    },
    Label {
        id: String,
        anchor: String,
        offset_x: f32,
        offset_y: f32,
        text: String,
        font_size: f32,
        color: [f32; 3],
    },
    Button {
        id: String,
        anchor: String,
        offset_x: f32,
        offset_y: f32,
        width: f32,
        height: f32,
        text: String,
        callback: String,
    },
    Image {
        id: String,
        texture_path: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        uv_rect: [f32; 4],
    },
    Slot {
        id: String,
        x: f32,
        y: f32,
        size: f32,
        texture_path: Option<String>,
        uv_rect: Option<[f32; 4]>,
        count_text: Option<String>,
        selected: bool,
        hotkey_label: Option<String>,
    },
    RttView {
        id: String,
        target_id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

impl UiElement {
    pub fn id(&self) -> &str {
        match self {
            UiElement::Panel { id, .. } => id,
            UiElement::Bar { id, .. } => id,
            UiElement::Label { id, .. } => id,
            UiElement::Button { id, .. } => id,
            UiElement::Image { id, .. } => id,
            UiElement::Slot { id, .. } => id,
            UiElement::RttView { id, .. } => id,
        }
    }
}

pub fn calculate_anchor_pos(
    anchor: &str,
    offset_x: f32,
    offset_y: f32,
    width: f32,
    height: f32,
    viewport_rect: Rect,
) -> Pos2 {
    let clean = anchor.to_lowercase().replace('-', "_");
    match clean.as_str() {
        "top_center" | "topcenter" => Pos2::new(
            viewport_rect.center().x - width * 0.5 + offset_x,
            viewport_rect.min.y + offset_y,
        ),
        "top_right" | "topright" => Pos2::new(
            viewport_rect.max.x - width - offset_x,
            viewport_rect.min.y + offset_y,
        ),
        "center_left" | "centerleft" => Pos2::new(
            viewport_rect.min.x + offset_x,
            viewport_rect.center().y - height * 0.5 + offset_y,
        ),
        "center" => Pos2::new(
            viewport_rect.center().x - width * 0.5 + offset_x,
            viewport_rect.center().y - height * 0.5 + offset_y,
        ),
        "center_right" | "centerright" => Pos2::new(
            viewport_rect.max.x - width - offset_x,
            viewport_rect.center().y - height * 0.5 + offset_y,
        ),
        "bottom_left" | "bottomleft" => Pos2::new(
            viewport_rect.min.x + offset_x,
            viewport_rect.max.y - height - offset_y,
        ),
        "bottom_center" | "bottomcenter" => Pos2::new(
            viewport_rect.center().x - width * 0.5 + offset_x,
            viewport_rect.max.y - height - offset_y,
        ),
        "bottom_right" | "bottomright" => Pos2::new(
            viewport_rect.max.x - width - offset_x,
            viewport_rect.max.y - height - offset_y,
        ),
        _ => Pos2::new(
            viewport_rect.min.x + offset_x,
            viewport_rect.min.y + offset_y,
        ), // "top_left" | "topleft"
    }
}

pub fn get_responsive_scale(viewport_rect: Rect) -> f32 {
    (viewport_rect.height() / 1080.0).clamp(0.5, 3.0)
}

pub fn calculate_anchor_pos_scaled(
    anchor: &str,
    offset_x: f32,
    offset_y: f32,
    width: f32,
    height: f32,
    viewport_rect: Rect,
    scale: f32,
) -> Pos2 {
    calculate_anchor_pos(
        anchor,
        offset_x * scale,
        offset_y * scale,
        width * scale,
        height * scale,
        viewport_rect,
    )
}

pub fn hit_test_ui(
    elements: &[UiElement],
    hidden_ids: &std::collections::HashSet<String>,
    viewport_rect: Rect,
    pointer_pos: Pos2,
) -> bool {
    let scale = get_responsive_scale(viewport_rect);
    for el in elements {
        if hidden_ids.contains(el.id()) {
            continue;
        }
        let rect = match el {
            UiElement::Button { anchor, offset_x, offset_y, width, height, .. } => {
                let p = calculate_anchor_pos_scaled(anchor, *offset_x, *offset_y, *width, *height, viewport_rect, scale);
                Rect::from_min_size(p, Vec2::new(*width * scale, *height * scale))
            }
            UiElement::Panel { anchor, offset_x, offset_y, width, height, .. } => {
                let p = calculate_anchor_pos_scaled(anchor, *offset_x, *offset_y, *width, *height, viewport_rect, scale);
                Rect::from_min_size(p, Vec2::new(*width * scale, *height * scale))
            }
            UiElement::Slot { x, y, size, .. } => {
                let p = Pos2::new(viewport_rect.min.x + *x * scale, viewport_rect.min.y + *y * scale);
                Rect::from_min_size(p, Vec2::splat(*size * scale))
            }
            _ => continue,
        };
        if rect.contains(pointer_pos) {
            return true;
        }
    }
    false
}

pub fn get_or_load_egui_texture(
    ctx: &egui::Context,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    path: &str,
) -> Option<egui::TextureHandle> {
    if let Some(handle) = texture_cache.get(path) {
        return Some(handle.clone());
    }

    let bytes = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
    let handle = ctx.load_texture(path, color_image, egui::TextureOptions::NEAREST);
    texture_cache.insert(path.to_string(), handle.clone());
    Some(handle)
}

#[allow(deprecated)]
pub fn render_declarative_ui(
    ctx: &egui::Context,
    painter: &egui::Painter,
    viewport_rect: Rect,
    elements: &[UiElement],
    pending_clicks: &Arc<Mutex<Vec<String>>>,
    texture_cache: &mut HashMap<String, egui::TextureHandle>,
    rtt_registry: Option<&crate::renderer::RenderTargetRegistry>,
) {
    let painter = painter.with_clip_rect(viewport_rect);
    for elem in elements {
        match elem {
            UiElement::Panel {
                anchor,
                offset_x,
                offset_y,
                width,
                height,
                color,
                ..
            } => {
                let pos = calculate_anchor_pos(anchor, *offset_x, *offset_y, *width, *height, viewport_rect);
                let rect = Rect::from_min_size(pos, Vec2::new(*width, *height));
                let fill = Color32::from_rgba_unmultiplied(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                    (color[3] * 255.0) as u8,
                );
                let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30));
                painter.rect(rect, CornerRadius::same(6), fill, stroke, StrokeKind::Inside);
            }
            UiElement::Bar {
                anchor,
                offset_x,
                offset_y,
                width,
                height,
                progress,
                label,
                color,
                ..
            } => {
                let pos = calculate_anchor_pos(anchor, *offset_x, *offset_y, *width, *height, viewport_rect);
                let rect = Rect::from_min_size(pos, Vec2::new(*width, *height));
                let bg_fill = Color32::from_rgba_unmultiplied(18, 22, 28, 210);
                let fg_fill = Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 45));
                painter.rect(rect, CornerRadius::same(4), bg_fill, stroke, StrokeKind::Inside);

                let progress_clamped = progress.clamp(0.0, 1.0);
                let fill_w = *width * progress_clamped;
                if fill_w > 0.0 {
                    let fill_rect = Rect::from_min_size(pos, Vec2::new(fill_w, *height));
                    painter.rect(fill_rect, CornerRadius::same(4), fg_fill, Stroke::NONE, StrokeKind::Inside);
                }

                if !label.is_empty() {
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                }
            }
            UiElement::Label {
                anchor,
                offset_x,
                offset_y,
                text,
                font_size,
                color,
                ..
            } => {
                let pos = calculate_anchor_pos(anchor, *offset_x, *offset_y, 0.0, 0.0, viewport_rect);
                let col = Color32::from_rgb(
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                );
                painter.text(
                    pos,
                    Align2::LEFT_TOP,
                    text,
                    FontId::proportional(*font_size),
                    col,
                );
            }
            UiElement::Button {
                id,
                anchor,
                offset_x,
                offset_y,
                width,
                height,
                text,
                callback,
            } => {
                let pos = calculate_anchor_pos(anchor, *offset_x, *offset_y, *width, *height, viewport_rect);
                let btn_area = egui::Area::new(egui::Id::new(format!("decl_ui_btn_{}", id)))
                    .fixed_pos(pos)
                    .constrain_to(viewport_rect);
                btn_area.show(ctx, |ui| {
                    ui.set_clip_rect(viewport_rect);
                    if ui.add_sized([*width, *height], egui::Button::new(RichText::new(text).size(14.0).strong())).clicked() {
                        if let Ok(mut clicks) = pending_clicks.lock() {
                            clicks.push(callback.clone());
                        }
                    }
                });
            }
            UiElement::Image {
                texture_path,
                x,
                y,
                width,
                height,
                uv_rect,
                ..
            } => {
                let px = if *x < 0.0 { viewport_rect.max.x + *x } else { viewport_rect.min.x + *x };
                let py = if *y < 0.0 { viewport_rect.max.y + *y } else { viewport_rect.min.y + *y };
                let img_rect = Rect::from_min_size(Pos2::new(px, py), Vec2::new(*width, *height));
                if let Some(tex) = get_or_load_egui_texture(ctx, texture_cache, texture_path) {
                    let uv = Rect::from_min_max(
                        Pos2::new(uv_rect[0], uv_rect[1]),
                        Pos2::new(uv_rect[2], uv_rect[3]),
                    );
                    painter.image(tex.id(), img_rect, uv, Color32::WHITE);
                }
            }
            UiElement::Slot {
                x,
                y,
                size,
                texture_path,
                uv_rect,
                count_text,
                selected,
                hotkey_label,
                ..
            } => {
                let px = if *x < 0.0 { viewport_rect.max.x + *x } else { viewport_rect.min.x + *x };
                let py = if *y < 0.0 { viewport_rect.max.y + *y } else { viewport_rect.min.y + *y };
                let slot_rect = Rect::from_min_size(Pos2::new(px, py), Vec2::splat(*size));

                // 1. Background & border
                let bg_fill = if *selected {
                    Color32::from_rgba_unmultiplied(42, 50, 68, 235)
                } else {
                    Color32::from_rgba_unmultiplied(20, 24, 32, 215)
                };
                let stroke = if *selected {
                    Stroke::new(2.5, Color32::from_rgb(255, 215, 0)) // Gold highlighted border
                } else {
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 45))
                };
                painter.rect(slot_rect, CornerRadius::same(5), bg_fill, stroke, StrokeKind::Inside);

                // 2. Icon (if texture_path is provided)
                if let Some(path) = texture_path {
                    if let Some(tex) = get_or_load_egui_texture(ctx, texture_cache, path) {
                        let icon_rect = slot_rect.shrink((*size * 0.12).max(2.0));
                        let uv = uv_rect
                            .map(|r| Rect::from_min_max(Pos2::new(r[0], r[1]), Pos2::new(r[2], r[3])))
                            .unwrap_or_else(|| Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)));
                        painter.image(tex.id(), icon_rect, uv, Color32::WHITE);
                    }
                }

                // 3. Hotkey Label (top-left)
                if let Some(label) = hotkey_label {
                    if !label.is_empty() {
                        painter.text(
                            slot_rect.min + Vec2::new(4.0, 3.0),
                            Align2::LEFT_TOP,
                            label,
                            FontId::proportional(11.0),
                            Color32::from_rgba_unmultiplied(210, 220, 235, 220),
                        );
                    }
                }

                // 4. Count Text (bottom-right with drop-shadow)
                if let Some(count) = count_text {
                    if !count.is_empty() {
                        let text_pos = slot_rect.max - Vec2::new(4.0, 3.0);
                        // Drop shadow
                        painter.text(
                            text_pos + Vec2::new(1.0, 1.0),
                            Align2::RIGHT_BOTTOM,
                            count,
                            FontId::proportional(13.0),
                            Color32::from_rgba_unmultiplied(0, 0, 0, 220),
                        );
                        // Text
                        painter.text(
                            text_pos,
                            Align2::RIGHT_BOTTOM,
                            count,
                            FontId::proportional(13.0),
                            Color32::WHITE,
                        );
                    }
                }
            }
            UiElement::RttView {
                target_id,
                x,
                y,
                width,
                height,
                ..
            } => {
                let px = if *x < 0.0 { viewport_rect.max.x + *x } else { viewport_rect.min.x + *x };
                let py = if *y < 0.0 { viewport_rect.max.y + *y } else { viewport_rect.min.y + *y };
                let rect = Rect::from_min_size(Pos2::new(px, py), Vec2::new(*width, *height));
                painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(10, 12, 16));
                if let Some(rtt) = rtt_registry.and_then(|r| r.get(target_id)) {
                    if let Some(tid) = rtt.egui_texture_id {
                        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                        painter.image(tid, rect, uv, Color32::WHITE);
                    }
                }
                painter.rect_stroke(rect, CornerRadius::same(4), Stroke::new(1.0, Color32::from_rgb(56, 189, 248)), StrokeKind::Outside);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_anchor_pos() {
        let viewport = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1000.0, 600.0));
        
        let p_top_left = calculate_anchor_pos("top_left", 10.0, 20.0, 100.0, 50.0, viewport);
        assert_eq!(p_top_left, Pos2::new(10.0, 20.0));

        let p_top_center = calculate_anchor_pos("top_center", 0.0, 10.0, 100.0, 50.0, viewport);
        assert_eq!(p_top_center, Pos2::new(450.0, 10.0));

        let p_top_right = calculate_anchor_pos("top_right", 10.0, 20.0, 100.0, 50.0, viewport);
        assert_eq!(p_top_right, Pos2::new(890.0, 20.0));

        let p_center_left = calculate_anchor_pos("center_left", 15.0, 0.0, 100.0, 60.0, viewport);
        assert_eq!(p_center_left, Pos2::new(15.0, 270.0));

        let p_center = calculate_anchor_pos("center", 0.0, 0.0, 200.0, 100.0, viewport);
        assert_eq!(p_center, Pos2::new(400.0, 250.0));

        let p_center_right = calculate_anchor_pos("center_right", 15.0, 0.0, 100.0, 60.0, viewport);
        assert_eq!(p_center_right, Pos2::new(885.0, 270.0));

        let p_bottom_left = calculate_anchor_pos("bottom_left", 10.0, 10.0, 100.0, 40.0, viewport);
        assert_eq!(p_bottom_left, Pos2::new(10.0, 550.0));

        let p_bottom_center = calculate_anchor_pos("bottom_center", 0.0, 10.0, 200.0, 40.0, viewport);
        assert_eq!(p_bottom_center, Pos2::new(400.0, 550.0));

        let p_bottom_right = calculate_anchor_pos("bottom_right", 10.0, 10.0, 100.0, 40.0, viewport);
        assert_eq!(p_bottom_right, Pos2::new(890.0, 550.0));

        // Test responsive scale
        let scale_1080 = get_responsive_scale(Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0)));
        assert!((scale_1080 - 1.0).abs() < 1e-4);

        let scale_720 = get_responsive_scale(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0)));
        assert!((scale_720 - (720.0 / 1080.0)).abs() < 1e-4);

        let p_scaled = calculate_anchor_pos_scaled("top_left", 10.0, 20.0, 100.0, 50.0, viewport, 2.0);
        assert_eq!(p_scaled, Pos2::new(20.0, 40.0));
    }
}
