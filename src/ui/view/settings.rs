use gpui::{ClickEvent, Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::{
    StyledExt,
    button::{Button, ButtonVariants},
    slider::Slider,
    switch::Switch,
};

use crate::ui::render::theme;
use crate::ui::view::TetrisView;

impl TetrisView {
    pub(super) fn render_settings_overlay(
        &self,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if !self.ui.show_settings {
            return div().hidden();
        }

        let card_width = (320.0 * scale).clamp(240.0, 420.0);
        let title_size = (22.0 * scale).clamp(18.0, 30.0);
        let body_size = (14.0 * scale).clamp(12.0, 18.0);

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::overlay_bg())
            .opacity(0.96)
            .child(
                div()
                    .w(px(card_width))
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::panel_bg())
                    .text_color(theme::panel_text())
                    .text_size(px(body_size))
                    .child(
                        div()
                            .text_size(px(title_size))
                            .font_semibold()
                            .child("Settings"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child("Sound effects")
                                    .child(
                                        Switch::new("settings-mute")
                                            .checked(self.ui.sfx_muted)
                                            .label("Mute")
                                            .on_click(cx.listener(Self::on_toggle_mute)),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child("Volume")
                                    .child(self.ui.sfx_volume_label()),
                            )
                            .child(Slider::new(&self.volume_slider).w_full()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                Button::new("settings-reset")
                                    .outline()
                                    .label("Reset")
                                    .on_click(cx.listener(Self::on_reset_settings)),
                            )
                            .child(
                                Button::new("settings-close")
                                    .primary()
                                    .label("Done")
                                    .on_click(cx.listener(Self::on_close_settings)),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::secondary_text())
                            .child("Keyboard: M mute · +/- volume · 0 reset · Esc close"),
                    ),
            )
    }

    pub(super) fn sync_volume_slider(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.volume_slider.update(cx, |slider, cx| {
            slider.set_value(self.ui.sfx_volume, window, cx);
        });
    }

    fn on_toggle_mute(&mut self, _: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.ui.toggle_mute();
        cx.notify();
    }

    fn on_reset_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.ui.reset_settings();
        self.sync_volume_slider(window, cx);
        cx.notify();
    }

    fn on_close_settings(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.ui.close_settings();
        self.focus_handle.focus(window);
        cx.notify();
    }
}
