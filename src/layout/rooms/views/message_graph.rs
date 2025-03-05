use std::{collections::HashMap, ops::RangeInclusive};

use crate::app::ChatApp;
use chrono::{DateTime, Local, Utc};
use egui::Color32;
use egui_plot::{AxisHints, BoxElem, BoxPlot, BoxSpread, GridMark, Legend, Plot, VLine};
pub struct MessageGraphView {}


trait AutoReset {
    fn auto_reset(self, auto: bool) -> Self;
}

impl <'a> AutoReset for Plot<'a>{
    fn auto_reset(self, auto: bool) -> Self {
        if auto{
           return self.reset()
        }
        self
    }
}

impl MessageGraphView {

    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        let now = Local::now().timestamp() as f64
            + Local::now().timestamp_subsec_millis() as f64 / 1000.0;

        let locked_model = app.model_arc.lock().unwrap();
        let mut per_sender = HashMap::new();

        for (index, message) in locked_model.messages.iter().enumerate() {
            let key = message.sender.uuid.clone();
            if !per_sender.contains_key(&key) {
                per_sender.insert(key, (message.sender.clone(), Vec::new()));
            }

            if let Some((_sender, box_elems)) = per_sender.get_mut(&message.sender.uuid) {
                let (tx, mut rx) = message.get_timestamps();

                // TODO : remove that
                rx = rx + 3.0;

                box_elems.push(
                    BoxElem::new(index as f64, BoxSpread::new(tx + 1.0, tx, tx, rx, rx - 1.0))
                        .name(message.text.clone()),
                );
            };
        }

        let time_formatter = |x: GridMark, _range: &RangeInclusive<f64>| {
            // Convert timestamp to readable datetime
            let datetime = DateTime::<Utc>::from_timestamp(x.value as i64, 0).unwrap_or(Utc::now());
            return datetime.format("%Y-%m-%d").to_string()
                + "\n"
                + &datetime.format("%H:%M:%S").to_string();
        };

        let x_axes = vec![AxisHints::new_x()
            .label("Time")
            .formatter(time_formatter)
            .placement(egui_plot::VPlacement::Top)];

        let reset_requested = ui.button("Reset view").clicked();
        Plot::new("Box Plot Demo")
            .legend(Legend::default())
            .allow_zoom(true)
            .allow_drag(true)
            .custom_x_axes(x_axes)
            .show_x(false)
            .show_y(false) // setting this to try would display the name (message text), maybe use something better
            .auto_reset(reset_requested)
            .show(ui, |plot_ui| {
                plot_ui.vline(VLine::new(now).color(Color32::from_rgb(255, 0, 0)));

                for (_uuid, (peer, boxes)) in per_sender {
                    let box_for_senders = BoxPlot::new(boxes)
                        .name(peer.name.clone())
                        .color(peer.get_color())
                        .horizontal();
                    plot_ui.box_plot(box_for_senders);
                }
            });
        let ctx = app.handler_arc.lock().unwrap().ctx.clone();
        ctx.request_repaint();

    }
}
