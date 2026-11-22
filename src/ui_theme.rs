use std::path::Path;
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Vec2, Visuals,
};

pub const COLOR_BG_PANEL: Color32 = Color32::from_rgb(0x18, 0x1A, 0x20);       // #181A20
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(0x20, 0x23, 0x2B);        // #20232B
pub const COLOR_BG_INPUT: Color32 = Color32::from_rgb(0x13, 0x15, 0x1A);       // #13151A
pub const COLOR_BORDER_SUBTLE: Color32 = Color32::from_rgb(0x2E, 0x33, 0x40);  // #2E3340
pub const COLOR_BORDER_FOCUSED: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8); // #38BDF8
pub const COLOR_TEXT_MAIN: Color32 = Color32::from_rgb(0xE6, 0xE8, 0xEE);      // #E6E8EE
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0x8E, 0x95, 0xA5);     // #8E95A5
pub const COLOR_ACCENT_PRIMARY: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8); // #38BDF8
pub const COLOR_ACCENT_HOVER: Color32 = Color32::from_rgb(0x0E, 0xA5, 0xE9);   // #0EA5E9
pub const COLOR_ACCENT_ACTIVE: Color32 = Color32::from_rgb(0x02, 0x84, 0xC7);  // #0284C7

// Axis Colors for studio-level Vector3 controls
pub const COLOR_AXIS_X: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44); // Red #EF4444
pub const COLOR_AXIS_Y: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E); // Green #22C55E
pub const COLOR_AXIS_Z: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6); // Blue #3B82F6

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let segoe_path = Path::new("C:\\Windows\\Fonts\\segoeui.ttf");
    if segoe_path.exists() {
        if let Ok(bytes) = std::fs::read(segoe_path) {
            fonts.font_data.insert(
                "segoeui".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "segoeui".to_owned());
        }
    }

    let consola_path = Path::new("C:\\Windows\\Fonts\\consola.ttf");
    if consola_path.exists() {
        if let Ok(bytes) = std::fs::read(consola_path) {
            fonts.font_data.insert(
                "consola".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "consola".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.override_text_color = Some(COLOR_TEXT_MAIN);
    visuals.panel_fill = COLOR_BG_PANEL;
    visuals.window_fill = COLOR_BG_CARD;
    visuals.faint_bg_color = COLOR_BG_CARD;
    visuals.extreme_bg_color = COLOR_BG_INPUT;
    visuals.code_bg_color = COLOR_BG_INPUT;

    visuals.window_stroke = Stroke::new(1.0, COLOR_BORDER_SUBTLE);
    visuals.window_corner_radius = CornerRadius::same(6);
    visuals.menu_corner_radius = CornerRadius::same(5);

    // Noninteractive widgets (frames, separators, labels)
    visuals.widgets.noninteractive.bg_fill = COLOR_BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, COLOR_BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, COLOR_TEXT_MAIN);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(5);

    // Inactive buttons, inputs, sliders
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x26, 0x2A, 0x34);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, COLOR_BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, COLOR_TEXT_MAIN);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(5);

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = COLOR_ACCENT_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, COLOR_BORDER_FOCUSED);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(5);

    // Active (pressed) widgets
    visuals.widgets.active.bg_fill = COLOR_ACCENT_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, COLOR_BORDER_FOCUSED);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = CornerRadius::same(5);

    // Open menus / dropdowns
    visuals.widgets.open = visuals.widgets.active;

    // Selection color
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(56, 189, 248, 60);
    visuals.selection.stroke = Stroke::new(1.0, COLOR_ACCENT_PRIMARY);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 4.0);
    style.spacing.window_margin = Margin::same(8);
    style.spacing.indent = 16.0;
    style.spacing.scroll.bar_width = 7.0;
    style.spacing.scroll.bar_inner_margin = 2.0;

    // Set proportional font sizes for clean hierarchy
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Name("Subheading".into()),
        FontId::new(15.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.5, FontFamily::Monospace),
    );

    ctx.set_style_of(egui::Theme::Dark, std::sync::Arc::new(style));
}

/// Helper to render an egui group as a modern styled card
pub fn studio_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(COLOR_BG_CARD)
        .stroke(Stroke::new(1.0, COLOR_BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::same(10))
        .show(ui, add_contents)
        .inner
}

/// Helper to render collapsible studio-style card
pub fn collapsible_card<R>(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<R> {
    studio_card(ui, |ui| {
        egui::CollapsingHeader::new(egui::RichText::new(title).strong().color(COLOR_TEXT_MAIN))
            .id_salt(id)
            .default_open(default_open)
            .show(ui, add_contents)
            .body_returned
    })
}

/// Studio Vector3 controls with color badges (X=Red, Y=Green, Z=Blue) and Reset button
pub fn ui_vec3_colored(
    ui: &mut egui::Ui,
    label: &str,
    values: &mut [f32; 3],
    reset_val: [f32; 3],
    speed: f32,
    range: Option<std::ops::RangeInclusive<f32>>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).strong().color(COLOR_TEXT_MAIN));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("[R]")
                .on_hover_text(format!(
                    "Reset to [{:.1}, {:.1}, {:.1}]",
                    reset_val[0], reset_val[1], reset_val[2]
                ))
                .clicked()
            {
                *values = reset_val;
                changed = true;
            }
        });
    });

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // X Component (Red badge)
        ui.label(egui::RichText::new("X").color(COLOR_AXIS_X).strong().size(11.0));
        let mut drag_x = egui::DragValue::new(&mut values[0]).speed(speed);
        if let Some(r) = &range {
            drag_x = drag_x.range(r.clone());
        }
        if ui.add(drag_x).changed() {
            changed = true;
        }

        ui.add_space(4.0);

        // Y Component (Green badge)
        ui.label(egui::RichText::new("Y").color(COLOR_AXIS_Y).strong().size(11.0));
        let mut drag_y = egui::DragValue::new(&mut values[1]).speed(speed);
        if let Some(r) = &range {
            drag_y = drag_y.range(r.clone());
        }
        if ui.add(drag_y).changed() {
            changed = true;
        }

        ui.add_space(4.0);

        // Z Component (Blue badge)
        ui.label(egui::RichText::new("Z").color(COLOR_AXIS_Z).strong().size(11.0));
        let mut drag_z = egui::DragValue::new(&mut values[2]).speed(speed);
        if let Some(r) = &range {
            drag_z = drag_z.range(r.clone());
        }
        if ui.add(drag_z).changed() {
            changed = true;
        }
    });

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_theme_application() {
        let ctx = egui::Context::default();
        setup_custom_fonts(&ctx);
        apply_theme(&ctx);

        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(visuals.panel_fill, COLOR_BG_PANEL);
        assert_eq!(visuals.window_fill, COLOR_BG_CARD);
        assert_eq!(visuals.extreme_bg_color, COLOR_BG_INPUT);
    }
}
