#[cfg(target_os = "windows")]
mod windows {
    use pollster::block_on;
    use rarog_compositor::{
        FrameCause, FrameDecision, FrameId, FramePlanner, FrameSubmission, OwnedFrameSubmission,
        PresentationStatus, PresentingCompositorWorker, SurfaceId, SurfaceSize,
    };
    use rarog_engine::{BaseUrl, Engine, View, ViewOptions};
    use rarog_platform_windows::{WindowsGpuError, WindowsPresentingCompositor};
    use rarog_types::{Point, Size};
    use std::error::Error;
    use std::fs;
    use std::io;
    use std::sync::Arc;
    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::{MouseScrollDelta, WindowEvent};
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
            compositor: None,
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
        compositor: Option<PresentingCompositorWorker<WindowsGpuError>>,
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
            let size = window.inner_size();
            let backend = block_on(WindowsPresentingCompositor::request(
                Arc::clone(&window),
                size.width,
                size.height,
            ))?;
            let compositor = PresentingCompositorWorker::spawn(backend)?;

            self.window = Some(Arc::clone(&window));
            self.compositor = Some(compositor);
            window.request_redraw();
            Ok(())
        }

        fn resize(&mut self) {
            self.view.request_frame(FrameCause::Resize);
        }

        fn present_retained(&mut self) -> Result<PresentationStatus, Box<dyn Error>> {
            let Some(compositor) = self.compositor.as_ref() else {
                return Ok(PresentationStatus::Deferred);
            };
            compositor.try_present_retained()?;
            let completion = compositor.recv_completion()?;
            if completion.frame().is_some() {
                return Err(
                    io::Error::other("retained presentation returned a frame completion").into(),
                );
            }
            let status = completion.result()?;
            self.request_redraw_if_deferred(status);
            Ok(status)
        }

        fn submit_and_present(
            &mut self,
            submission: OwnedFrameSubmission,
        ) -> Result<PresentationStatus, Box<dyn Error>> {
            let expected = submission.frame_id();
            let Some(compositor) = self.compositor.as_ref() else {
                return Ok(PresentationStatus::Deferred);
            };
            compositor.try_submit_and_present(submission)?;
            let completion = compositor.recv_completion()?;
            if completion.frame() != Some(expected) {
                return Err(completion_mismatch(expected, completion.frame()).into());
            }
            let status = completion.result()?;
            self.request_redraw_if_deferred(status);
            Ok(status)
        }

        fn request_redraw_if_deferred(&self, status: PresentationStatus) {
            if status == PresentationStatus::Deferred {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }

        fn render(&mut self) -> Result<(), Box<dyn Error>> {
            let Some(window) = self.window.as_ref() else {
                return Ok(());
            };
            let size = window.inner_size();
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            let Some(request) = self.view.begin_frame_request()? else {
                self.present_retained()?;
                return Ok(());
            };
            let request_id = request.id();
            let outcome = self.render_scheduled(size.width, size.height, request.primary_cause());

            match outcome {
                Ok(PresentationStatus::Presented) => {
                    self.view.complete_frame_request(request_id)?;
                    Ok(())
                }
                Ok(PresentationStatus::Deferred) => {
                    self.view.discard_frame_request(request_id)?;
                    Ok(())
                }
                Err(error) => {
                    self.view.discard_frame_request(request_id)?;
                    Err(error)
                }
            }
        }

        fn render_scheduled(
            &mut self,
            width: u32,
            height: u32,
            cause: FrameCause,
        ) -> Result<PresentationStatus, Box<dyn Error>> {
            let surface_size = SurfaceSize::new(width, height);
            let frame = self.view.render(Size {
                width: width as f32,
                height: height as f32,
            })?;
            let decision =
                frame.plan_compositor_frame_with_cause(&mut self.planner, surface_size, cause)?;

            match decision {
                FrameDecision::Noop => self.present_retained(),
                FrameDecision::Suspended { .. } => Ok(PresentationStatus::Deferred),
                FrameDecision::Submit(plan) => {
                    let id = plan.id();
                    let submission = OwnedFrameSubmission::from_borrowed(FrameSubmission {
                        plan: &plan,
                        display_list: frame.display_list,
                        image_resources: Some(frame.image_resources),
                        viewport_translation: frame.viewport_translation,
                        clear_color: frame.clear_color,
                    });
                    let outcome = self.submit_and_present(submission);

                    match outcome {
                        Ok(PresentationStatus::Presented) => {
                            self.planner.complete(id)?;
                            Ok(PresentationStatus::Presented)
                        }
                        Ok(PresentationStatus::Deferred) => {
                            self.planner.discard(id)?;
                            Ok(PresentationStatus::Deferred)
                        }
                        Err(error) => {
                            self.planner.discard(id)?;
                            Err(error)
                        }
                    }
                }
            }
        }

        fn scroll(&mut self, delta: MouseScrollDelta) -> Result<(), Box<dyn Error>> {
            if self.view.root_scroll_node().is_none() {
                return Ok(());
            }

            let delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => Point {
                    x: -x * 40.0,
                    y: -y * 40.0,
                },
                MouseScrollDelta::PixelDelta(position) => Point {
                    x: -(position.x as f32),
                    y: -(position.y as f32),
                },
            };
            let changed = self.view.scroll_root_by(delta)?;
            if changed.changed() {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Ok(())
        }

        fn fail(&self, event_loop: &ActiveEventLoop, error: &dyn std::fmt::Display) {
            eprintln!("rarog-window: {error}");
            event_loop.exit();
        }
    }

    fn completion_mismatch(expected: FrameId, actual: Option<FrameId>) -> io::Error {
        io::Error::other(format!(
            "compositor completion mismatch: expected {expected:?}, got {actual:?}"
        ))
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
                WindowEvent::Resized(_) => {
                    self.resize();
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    if let Err(error) = self.scroll(delta) {
                        self.fail(event_loop, error.as_ref());
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
