use rukoh::{Colour, Rukoh, RukohConfig, WindowMode};

fn main() -> Result<(), rukoh::Error> {
    let mut app = Rukoh::new(RukohConfig {
        title: "rukoh — hello window",
        window_mode: WindowMode::Windowed {
            width: 800,
            height: 600,
        },
        vsync: true,
        ..Default::default()
    })?;

    while let Some(mut frame) = app.next_frame() {
        // Print delta-time so we can verify the loop is running correctly.
        println!(
            "dt = {:.4}s  ({:.1} fps)",
            frame.delta_time(),
            1.0 / frame.delta_time().max(f32::EPSILON)
        );

        frame.clear(Colour::CORNFLOWER_BLUE);

        // Frame drops here → IDXGISwapChain::Present
    }

    println!("Window closed — exiting.");
    Ok(())
}
