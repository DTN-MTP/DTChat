use std::{collections::HashMap, ops::RangeInclusive};

use crate::app::ChatApp;
use chrono::{DateTime, Local, Utc};
use egui::{Color32, Vec2b};
use egui_plot::{AxisHints, BoxElem, BoxPlot, BoxSpread, GridMark, Legend, Plot, VLine};
pub struct MessageGraphView {}

trait AutoReset {
    fn auto_reset(self, auto: bool) -> Self;
}

impl<'a> AutoReset for Plot<'a> {
    fn auto_reset(self, auto: bool) -> Self {
        if auto {
            return self.reset();
        }
        self
    }
}

pub fn ts_to_str(
    datetime: &DateTime<Utc>,
    date: bool,
    time: bool,
    separator: Option<String>,
) -> String {
    let mut res = "".to_string();
    if date {
        res += &datetime.format("%Y-%m-%d").to_string();
    }
    if let Some(sep) = separator {
        res += &sep;
    }
    if time {
        res += &datetime.format("%H:%M:%S").to_string()
    }
    return res;
}

impl MessageGraphView {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
        let now = Local::now().timestamp_millis() as f64;
        // + Local::now().timestamp_subsec_millis() as f64 / 1000.0;

        let locked_model = app.model_arc.lock().unwrap();
        let mut per_sender = HashMap::new();

        for (index, message) in locked_model.messages.iter().enumerate() {
            let key = message.sender.uuid.clone();
            if !per_sender.contains_key(&key) {
                per_sender.insert(key, (message.sender.clone(), Vec::new()));
            }

            if let Some((_sender, box_elems)) = per_sender.get_mut(&message.sender.uuid) {
                let (tx, pbat_opt, rx_opt) = message.get_timestamps();


                let upper_whisker =  if let Some(received) = rx_opt {
                    received - 1.0
                } else {
                    if let Some(pbat) = pbat_opt {
                        pbat
                    } else {
                        tx - 1.0
                    }
                };

                box_elems.push(
                    BoxElem::new(index as f64, BoxSpread::new(tx + 1.0, tx, tx, rx_opt.unwrap_or(tx), upper_whisker))
                        .name(message.text.clone()),
                );
            };
        }

        let time_formatter = |x: GridMark, _range: &RangeInclusive<f64>| {
            // Convert timestamp to readable datetime
            let datetime = DateTime::<Utc>::from_timestamp_millis(x.value as i64).unwrap();
            return ts_to_str(&datetime, true, true, Some("\n".to_string()));
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
            .include_y((locked_model.messages.len() + 1) as f64)
            .custom_y_axes(vec![])
            .allow_scroll(Vec2b { x: true, y: false })
            .allow_drag(Vec2b { x: true, y: false })
            .show_x(true)
            .show_y(false) // setting this to true would display the name (message text), maybe use something better
            .label_formatter(|name, value| {
                if !name.is_empty() {
                    format!("{}: {:.*}%", name, 1, value.y)
                } else {
                    let value = DateTime::<Utc>::from_timestamp_millis(value.x as i64).unwrap();
                    format!("{}", ts_to_str(&value, false, true, None))
                }
            })
            .auto_reset(reset_requested)
            .show(ui, |plot_ui| {
                plot_ui.vline(VLine::new("Current Time", now).color(Color32::from_rgb(255, 0, 0)));

                for (_uuid, (peer, boxes)) in per_sender {
                    let peer_name = peer.name.clone();

                    // Create a new String that we can move into the closure
                    let formatter_peer_name = peer_name.clone();

                    let box_for_senders = BoxPlot::new(peer_name.clone(), boxes)
                        .color(peer.get_color())
                        .horizontal()
                        .allow_hover(true)
                        .element_formatter(Box::new(move |bar, _bar_chart| {
                            let tx_time =
                                DateTime::<Utc>::from_timestamp_millis(bar.spread.quartile1 as i64)
                                    .unwrap();
                            let rx_time =
                                DateTime::<Utc>::from_timestamp_millis(bar.spread.quartile3 as i64)
                                    .unwrap();
                            let date = tx_time.date_naive() != rx_time.date_naive();
                            format!(
                                "Message: {}\nSent by {}\ntx time: {}\nrx_time: {}",
                                bar.name,
                                formatter_peer_name,
                                ts_to_str(&tx_time, date, true, None),
                                ts_to_str(&rx_time, date, true, None),
                            )
                        }));

                    plot_ui.box_plot(box_for_senders);
                }
            });

        let ctx = app.handler_arc.lock().unwrap().ctx.clone();
        ctx.request_repaint();
    }
}
