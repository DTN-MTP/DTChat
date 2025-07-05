// use crate::app::ChatApp;
// use eframe::egui;

#[derive(Clone, Debug, PartialEq, Default)]
pub enum NavigationItems {
    #[default]
    Rooms,
    Contacts,
}

pub struct Header {}

impl Header {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(&mut self, ui: &mut eframe::egui::Ui) {
        ui.add_space(5.0);
        
        // Logo/Title section - aligned left
        ui.horizontal(|ui| {
            ui.add_space(10.0); // Small left margin
            
            // Application title with emphasis
            ui.label(
                eframe::egui::RichText::new("📡 DTChat")
                    .size(20.0)
                    .strong()
                    .color(eframe::egui::Color32::from_rgb(0, 122, 204))
            );
            
            ui.add_space(10.0); // Space between title and subtitle
            
            // Subtitle/tagline on the same line
            ui.label(
                eframe::egui::RichText::new("• Secure • Decentralized • Resilient")
                    .size(10.0)
                    .italics()
                    .color(eframe::egui::Color32::DARK_GRAY)
            );
        });
        
        ui.add_space(5.0);
    }
}

// pub struct MenuBar {}

// impl MenuBar {
//     pub fn new() -> Self {
//         Self {}
//     }

//     pub fn show(&mut self, app: &mut ChatApp, ui: &mut egui::Ui) {
//         ui.add_space(10.0);
//         ui.horizontal(|ui| {
//             ui.selectable_value(&mut app.context_menu, NavigationItems::Rooms, "Rooms");
//             ui.selectable_value(&mut app.context_menu, NavigationItems::Contacts, "Contacts");
//         });
//         ui.add_space(10.0);
//     }
// }
