#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    if let Err(err) = windows_launcher::run() {
        windows_launcher::write_error_log(&err.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!(
        "Paper Composer now uses the macOS WebKit app bundle. Run ./scripts/build_macos_app.sh or open Paper Composer.app."
    );
}

#[cfg(windows)]
mod windows_launcher {
    use std::{
        env, fs,
        net::TcpStream,
        path::{Path, PathBuf},
        process::{Child, Command},
        thread,
        time::Duration,
    };

    use std::os::windows::process::CommandExt;
    use winit::{
        application::ApplicationHandler,
        dpi::LogicalSize,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, EventLoop},
        window::{Window, WindowId},
    };
    use wry::{WebView, WebViewBuilder};

    const BRIDGE_ADDR: &str = "127.0.0.1:53683";
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let app_dir = env::current_exe()?
            .parent()
            .map(Path::to_path_buf)
            .ok_or("Could not find application directory")?;

        let bridge = first_existing(&[
            app_dir.join("drive_bridge.exe"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/drive_bridge.exe"),
        ])
        .ok_or("Missing drive_bridge.exe. Run scripts\\package_windows.ps1 first.")?;

        let index_html = first_existing(&[
            app_dir.join("web/index.html"),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/index.html"),
        ])
        .ok_or("Missing web\\index.html. Run scripts\\package_windows.ps1 first.")?;

        let bridge_child = if TcpStream::connect(BRIDGE_ADDR).is_err() {
            Some(
                Command::new(&bridge)
                    .current_dir(bridge.parent().unwrap_or(&app_dir))
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?,
            )
        } else {
            None
        };

        if bridge_child.is_some() {
            thread::sleep(Duration::from_millis(700));
        }

        let url = url::Url::from_file_path(&index_html)
            .map_err(|_| format!("Could not convert path to file URL: {}", index_html.display()))?;

        let event_loop = EventLoop::new()?;
        let mut app = PaperComposerApp {
            url: url.to_string(),
            bridge_child,
            window: None,
            webview: None,
        };
        event_loop.run_app(&mut app)?;
        stop_bridge(&mut app.bridge_child);
        Ok(())
    }

    fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
        paths.iter().find(|path| path.exists()).cloned()
    }

    pub fn write_error_log(message: &str) {
        let base = app_data_dir();
        let _ = fs::create_dir_all(&base);
        let _ = fs::write(base.join("launcher.log"), message);
    }

    fn stop_bridge(bridge_child: &mut Option<Child>) {
        if let Some(child) = bridge_child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    struct PaperComposerApp {
        url: String,
        bridge_child: Option<Child>,
        window: Option<Window>,
        webview: Option<WebView>,
    }

    impl ApplicationHandler for PaperComposerApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }

            let attrs = Window::default_attributes()
                .with_title("Paper Composer")
                .with_inner_size(LogicalSize::new(1440.0, 900.0))
                .with_min_inner_size(LogicalSize::new(1120.0, 720.0));

            let result = (|| -> Result<(Window, WebView), Box<dyn std::error::Error>> {
                let window = event_loop.create_window(attrs)?;
                let webview = WebViewBuilder::new().with_url(&self.url).build(&window)?;
                Ok((window, webview))
            })();

            match result {
                Ok((window, webview)) => {
                    self.window = Some(window);
                    self.webview = Some(webview);
                }
                Err(err) => {
                    write_error_log(&err.to_string());
                    stop_bridge(&mut self.bridge_child);
                    event_loop.exit();
                }
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            if matches!(event, WindowEvent::CloseRequested) {
                stop_bridge(&mut self.bridge_child);
                event_loop.exit();
            }
        }
    }

    fn app_data_dir() -> PathBuf {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("Paper Composer")
    }
}
