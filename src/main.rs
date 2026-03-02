use axum::{
    Router, extract::State, http::StatusCode, response::IntoResponse, routing::get, routing::post,
};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use zxing_app::run_app;

#[derive(Clone)]
struct AppState {
    file_handle: Arc<Mutex<Option<File>>>,
}
#[tokio::main]
async fn main() {
    //初始化状态数据
    let args: Vec<String> = std::env::args().collect();
    let initial_file = if args.len() > 1 {
        let path = &args[1];
        match File::options()
            .read(true)
            .write(true)
            .create(false)
            .open(path)
            .await
        {
            Ok(f) => Some(f),
            Err(_e) => {
                eprintln!("文件打开失败");
                None
            }
        }
    } else {
        eprintln!("未提供文件路径");
        None
    };
    let state = AppState {
        file_handle: Arc::new(Mutex::new(initial_file)),
    };
    //路由
    let router = Router::new()
        .route("/refresh", get(refresh_handler))
        .route("/save", post(save_handler))
        .with_state(state);
    //启动
    run_app("/home/zx/下载/demo1/web", router).await;
}

async fn refresh_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut guard = state.file_handle.lock().await;

    match &mut *guard {
        Some(file) => {
            if let Err(_e) = file.rewind().await {
                return error("重置指针失败");
            }
            let mut contents = String::new();
            match file.read_to_string(&mut contents).await {
                Ok(_) => (StatusCode::OK, contents),
                Err(_e) => return error("读取内容失败"),
            }
        }
        None => error("未提供文件路径"),
    }
}

async fn save_handler(State(state): State<AppState>, body: String) -> impl IntoResponse {
    let mut guard = state.file_handle.lock().await;

    let file = match &mut *guard {
        Some(f) => f,
        None => {
            return error("未提供文件路径");
        }
    };
    if let Err(_e) = file.set_len(0).await {
        return error("清空文件失败");
    }
    if let Err(_e) = file.rewind().await {
        return error("重置指针失败");
    }
    if let Err(_e) = file.write_all(body.as_bytes()).await {
        return error("写入文件失败");
    }
    if let Err(_e) = file.flush().await {
        return error("刷新缓冲区失败");
    }
    (StatusCode::OK, "Ok".to_string())
}
fn error(txt: &str) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", txt))
}
