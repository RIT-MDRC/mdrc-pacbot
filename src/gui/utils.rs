//! Utilities for GUI layouts.

use egui::{Align, InnerResponse, Layout, Ui};

/// Render a sub-UI as a group that is horizontally centered within the remaining width.
///
/// There is a bit of a paradox when implementing this. The horizontal offset for the start of the
/// group must be known before drawing the first contained widget. But the horizontal offset depends
/// on the group's total width, which is not known until after all the widgets have been drawn.
///
/// The solution here is that on the first frame, the group will not be drawn centered. Its width
/// will then be recorded and used on subsequent frames to center the group. If the width changes,
/// the centering will similarly be incorrect for a single frame.
pub fn centered_group<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    // ui.menu_button(title, add_contents)
    // ui.collapsing(heading, add_contents);

    let id = ui.id();
    let previous_width = ui.ctx().data_mut(|d| d.get_temp::<f32>(id));

    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        // If this is the first frame, visually hide the contents.
        ui.set_visible(previous_width.is_some());

        // Apply an X offset based on the width of the content on the last frame (if known).
        if let Some(previous_width) = previous_width {
            let x_offset = (ui.available_width() - previous_width) / 2.0;
            ui.add_space(x_offset);
        }

        // Add the contents and record the width.
        let x_start = ui.cursor().left();
        let res = add_contents(ui);
        let x_end = ui.cursor().left() - ui.style().spacing.item_spacing.x;
        let width = x_end - x_start;

        // Store the width for the next frame.
        ui.ctx().data_mut(|d| d.insert_temp(id, width));

        // If the width has changed, request a repaint.
        if previous_width != Some(width) {
            ui.ctx().request_repaint();
        }

        res
    })
}
