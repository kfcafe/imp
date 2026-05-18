fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("imp workbench")
            .with_inner_size([1280.0, 860.0]),
        ..Default::default()
    };

    eframe::run_native(
        "imp workbench",
        native_options,
        Box::new(|creation_context| Ok(Box::new(imp_gui::ImpGuiApp::new(creation_context)))),
    )
}
