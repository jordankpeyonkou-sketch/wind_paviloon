use std::{error::Error, sync::Arc};

use glassdesk_render::{until_next_minute, Renderer};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

#[cfg(windows)]
mod desktop;

pub fn run(desktop_mode: bool) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("GlassDesk")
            .with_inner_size(PhysicalSize::new(460, 190))
            .with_resizable(true)
            .with_decorations(false)
            .with_transparent(true)
            .build(&event_loop)?,
    );

    #[cfg(windows)]
    if desktop_mode {
        desktop::attach_to_desktop(&window)?;
    }

    #[cfg(not(windows))]
    if desktop_mode {
        return Err("--desktop mode is only supported on Windows.".into());
    }

    let mut renderer = pollster::block_on(Renderer::new(Arc::clone(&window)))?;
    let mut next_redraw = std::time::Instant::now();
    eprintln!(
        "GlassDesk is running{}; press Escape to close.",
        if desktop_mode { " on the desktop" } else { "" }
    );

    event_loop.run(move |event, event_loop| {
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw));

        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    renderer.resize(size);
                    renderer.request_redraw();
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed
                        && event.logical_key == Key::Named(NamedKey::Escape) =>
                {
                    event_loop.exit();
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } if !desktop_mode => {
                    if let Err(error) = window.drag_window() {
                        eprintln!("Could not move the GlassDesk window: {error}");
                    }
                }
                WindowEvent::RedrawRequested => match renderer.render() {
                    Ok(()) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        renderer.resize(window.inner_size());
                        next_redraw = std::time::Instant::now() + std::time::Duration::from_secs(1);
                        event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw));
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        eprintln!("The graphics surface timed out; the next frame will retry.");
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        eprintln!("The graphics adapter ran out of memory; GlassDesk is closing.");
                        event_loop.exit();
                    }
                },
                _ => {}
            },
            Event::AboutToWait if std::time::Instant::now() >= next_redraw => {
                renderer.request_redraw();
                next_redraw = std::time::Instant::now() + until_next_minute();
            }
            _ => {}
        }
    })?;

    Ok(())
}
