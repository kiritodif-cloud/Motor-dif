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
}

impl UiElement {
    pub fn id(&self) -> &str {
        match self {
            UiElement::Panel { id, .. } => id,
            UiElement::Bar { id, .. } => id,
            UiElement::Label { id, .. } => id,
            UiElement::Button { id, .. } => id,
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
    match anchor.to_lowercase().as_str() {
        "top_center" => Pos2::new(
            viewport_rect.center().x - width * 0.5 + offset_x,
            viewport_rect.min.y + offset_y,
        ),
        "top_right" => Pos2::new(
            viewport_rect.max.x - width - offset_x,
            viewport_rect.min.y + offset_y,
        ),
        "center" => Pos2::new(
            viewport_rect.center().x - width * 0.5 + offset_x,
            viewport_rect.center().y - height * 0.5 + offset_y,
        ),
        "bottom_left" => Pos2::new(
            viewport_rect.min.x + offset_x,
            viewport_rect.max.y - height - offset_y,
        ),
        "bottom_center" => Pos2::new(
            viewport_rect.center().x - width * 0.5 + offset_x,
            viewport_rect.max.y - height - offset_y,
        ),
        "bottom_right" => Pos2::new(
            viewport_rect.max.x - width - offset_x,
            viewport_rect.max.y - height - offset_y,
        ),
        _ => Pos2::new(
            viewport_rect.min.x + offset_x,
            viewport_rect.min.y + offset_y,
        ), // "top_left"
    }
}

#[allow(deprecated)]
pub fn render_declarative_ui(
    ctx: &egui::Context,
    painter: &egui::Painter,
    viewport_rect: Rect,
    elements: &[UiElement],
    pending_clicks: &Arc<Mutex<Vec<String>>>,
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

        let p_top_right = calculate_anchor_pos("top_right", 10.0, 20.0, 100.0, 50.0, viewport);
        assert_eq!(p_top_right, Pos2::new(890.0, 20.0));

        let p_center = calculate_anchor_pos("center", 0.0, 0.0, 200.0, 100.0, viewport);
        assert_eq!(p_center, Pos2::new(400.0, 250.0));

        let p_bottom_center = calculate_anchor_pos("bottom_center", 0.0, 10.0, 200.0, 40.0, viewport);
        assert_eq!(p_bottom_center, Pos2::new(400.0, 550.0));
    }
}
