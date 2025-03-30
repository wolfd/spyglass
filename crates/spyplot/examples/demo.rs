use spyplot::Spyplot;

fn main() {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Spyplot Demo",
        native_options,
        Box::new(|cc| Ok(Box::new(Spyplot::new(cc).unwrap()))),
    )
    .unwrap();
}
