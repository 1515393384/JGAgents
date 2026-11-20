//! 架阁 · Agent 客户端外壳。
//!
//! 只做两件事：开一个窗口；把 `web/dist` 通过 `jiage://` 协议原样端给 WebView。
//! 不做 IPC、不做命令总线 —— 业务全在前端，内核在 `agent-core`。

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::http::{Request, Response};
use wry::WebViewBuilder;

const SCHEME: &str = "jiage";

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("架阁 · 上下文不重算")
        .with_inner_size(LogicalSize::new(1180.0, 780.0))
        .with_min_inner_size(LogicalSize::new(760.0, 520.0))
        .build(&event_loop)
        .expect("窗口创建失败");

    let dist = dist_dir();
    eprintln!("[jiage] 前端目录 {}", dist.display());

    let load = match std::env::var("JIAGE_DEV_URL").ok() {
        Some(url) => url,
        None => format!("{SCHEME}://localhost/"),
    };

    let _webview = WebViewBuilder::new()
        .with_custom_protocol(SCHEME.into(), move |_id, req| serve(&dist, &req))
        .with_url(load)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// 前端产物目录：`JIAGE_DIST` → 当前目录下 `web/dist` → 与 web 工程并排的默认位置。
fn dist_dir() -> PathBuf {
    if let Ok(p) = std::env::var("JIAGE_DIST") {
        return PathBuf::from(p);
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("web").join("dist");
        if p.join("index.html").is_file() {
            return p;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../web/dist")
}

fn serve(root: &Path, req: &Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let rel = PathBuf::from(req.uri().path().trim_start_matches('/'));
    let escaped = rel
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_)));

    let mut file = if rel.as_os_str().is_empty() || escaped {
        root.join("index.html")
    } else {
        root.join(&rel)
    };
    if !file.is_file() {
        // 前端是单页路由，路径找不到就交回 index.html
        file = root.join("index.html");
    }

    match std::fs::read(&file) {
        Ok(bytes) => Response::builder()
            .header("Content-Type", mime(&file))
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(bytes)),
        Err(_) => Response::builder()
            .status(404)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Cow::Borrowed(&b"not found"[..])),
    }
    .expect("响应构造失败")
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}
