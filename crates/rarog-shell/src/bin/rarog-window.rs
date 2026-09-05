#[cfg(target_os = "windows")]
mod windows {
    use pollster::block_on;
    use rarog_compositor::{
        CompositorBackend, FrameDecision, FramePlanner, FrameSubmission, SurfaceId, SurfaceSize,
    };
    use rarog_compositor_wgpu::WgpuCompositorBackend;
    use rarog_engine::{BaseUrl, Engine, View, ViewOptions};
    use rarog_platform_windows::{WindowsGpuDevice, WindowsGpuSurface};
    use rarog_types::Size;
    use std::error::Error;
    use std::fs;
    use std::sync::Arc;
    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{Window, WindowId};

    pub fn run() -> Result<(), Box<dyn Error>> {
        let input = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "examples/hello.html".into());
        let source = fs::read_to_string(&input)?;
        let engine = Engine::builder().build()?;
        let mut view = engine.create_view(ViewOptions::default())?;
        view.load_html(source, BaseUrl::about_blank())?;

        let surface_id = SurfaceId::new(1).expect("non-zero bootstrap surface id");
        let mut app = WindowApp {
            input,
            view,
            window: None,
            gpu: None,
            surface: None,
            backend: None,
            planner: FramePlanner::new(surface_id),
        };
        let event_loop = EventLoop::new()?;
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    struct WindowApp {
        input: String,
        view: View,
        window: Option<Arc<Window>>,
        gpu: Option<WindowsGpuDevice>,
        surface: Option<WindowsGpuSurface>,
        backend: Option<WgpuCompositorBackend>,
        planner: FramePlanner,
    }

    impl WindowApp {
        fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
            if let Some(window) = &self.window {
                window.request_redraw();
                return Ok(());
            }

            let attributes = Window::default_attributes()
                .with_title(format!("Rarog GPU — {}", self.input))
                .with_inner_size(LogicalSize::new(1024.0, 768.0));
            let window = Arc::new(event_loop.create_window(attributes)?);
            let gpu = block_on(WindowsGpuDevice::request())?;
            let size = window.inner_size();
            let surface = gpu.create_surface(Arc::clone(&window), size.width, size.height)?;
            let backend = gpu.compositor_backend();

            self.window = Some(Arc::clone(&window));
            self.gpu = Some(gpu);
            self.surface = Some(surface);
            self.backend = Some(backend);
            window.request_redraw();
            Ok(())
        }

        fn resize(&mut self, width: u32, height: u32) -> Result<(), Box<dyn Error>> {
            let (Some(gpu), Some(surface)) = (self.gpu.as_ref(), self.surface.as_mut()) else {
                return Ok(());
            };
            surface.resize(gpu, width, height)?;
            Ok(())
        }

        fn render(&mut self) -> Result<(), Box<dyn Error>> {
            let Some(window) = self.window.as_ref() else {
                return Ok(());
            };
            let size = window.inner_size();
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            let surface_size = SurfaceSize::new(size.width, size.height);
            let frame = self.view.render(Size {
                width: size.width as f32,
                height: size.height as f32,
            })?;
            let decision = frame.plan_compositor_frame(&mut self.planner, surface_size)?;

            match decision {
                FrameDecision::Noop => {
                    let (Some(surface), Some(backend)) =
                        (self.surface.as_ref(), self.backend.as_mut())
                    else {
                        return Ok(());
                    };
                    surface.present(backend)?;
                }
                FrameDecision::Suspended { .. } => {}
                FrameDecision::Submit(plan) => {
                    let id = plan.id();
                    let Some(backend) = self.backend.as_mut() else {
                        self.planner.discard(id)?;
                        return Ok(());
                    };
                    if let Err(error) = backend.submit(FrameSubmission {
                        plan: &plan,
                        display_list: frame.display_list,
                        image_resources: Some(frame.image_resources),
                        clear_color: frame.clear_color,
                    }) {
                        self.planner.discard(id)?;
                        return Err(Box::new(error));
                    }

                    let Some(surface) = self.surface.as_ref() else {
                        self.planner.discard(id)?;
                        return Ok(());
                    };
                    if let Err(error) = surface.present(backend) {
                        self.planner.discard(id)?;
                        return Err(Box::new(error));
                    }
                    self.planner.complete(id)?;
                }
            }
            Ok(())
        }

        fn fail(&self, event_loop: &ActiveEventLoop, error: &dyn std::fmt::Display) {
            eprintln!("rarog-window: {error}");
            event_loop.exit();
        }
    }

    impl ApplicationHandler for WindowApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if let Err(error) = self.initialize(event_loop) {
                self.fail(event_loop, error.as_ref());
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            window_id: WindowId,
            event: WindowEvent,
        ) {
            if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
                return;
            }

            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    if let Err(error) = self.resize(size.width, size.height) {
                        self.fail(event_loop, error.as_ref());
                        return;
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Err(error) = self.render() {
                        self.fail(event_loop, error.as_ref());
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn main() -> std::process::ExitCode {
    match windows::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rarog-window: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn main() -> std::process::ExitCode {
    eprintln!("rarog-window is available only on Windows");
    std::process::ExitCode::FAILURE
}
