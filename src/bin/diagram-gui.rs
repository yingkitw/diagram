//! Desktop GUI entry point. Build with `cargo run --bin diagram-gui --features gui`.

fn main() -> eframe::Result<()> {
    diagram::gui::run()
}
