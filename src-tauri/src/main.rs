// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotenv::dotenv;
use perf_mon_lib::{collect_perf_info, get_buffer_status, BufferStatus, PerfInfo, DATA_BUFFER};
use reqwest::Client;
use std::env;
use tokio::time::{sleep, Duration};

/// Обертка для команды Tauri
#[tauri::command]
fn collect_perf_info_command() -> PerfInfo {
    collect_perf_info()
}

/// Добавим новую команду
#[tauri::command]
fn get_status() -> BufferStatus {
    get_buffer_status()
}

/// Изменим функцию отправки
async fn send_batch(batch: Vec<PerfInfo>) -> Result<(), reqwest::Error> {
    let client = Client::new();
    let json = serde_json::to_string(&batch).expect("Ошибка сериализации данных в JSON");

    let server_url =
        env::var("SERVER_URL").unwrap_or_else(|_| "http://yourserver.com/api/monitor".to_string());

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .body(json)
        .send()
        .await?;
    println!("Пакет отправлен, статус: {}", response.status());
    Ok(())
}

fn main() {
    println!("Запуск программы");
    dotenv().ok();
    println!("Загружены переменные окружения");
    // Создаём runtime для мониторинга
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    // Запускаем задачу мониторинга в отдельном потоке
    std::thread::spawn(move || {
        rt.block_on(async {
            let collect_interval = Duration::from_secs(30);
            println!("Задача мониторинга запущена");
            loop {
                println!("=== Новый цикл сбора данных ===");

                let snapshot = collect_perf_info();
                println!("Собран снапшот: {:?}", snapshot.time);

                {
                    let mut buffer = DATA_BUFFER.lock().unwrap();
                    buffer.push(snapshot);
                    println!("Размер буфера: {}", buffer.len());

                    if buffer.len() >= 50 {
                        let batch = buffer.drain(0..50).collect::<Vec<_>>();
                        tokio::spawn(async move {
                            if let Err(e) = send_batch(batch).await {
                                eprintln!("Ошибка отправки пакета: {:?}", e);
                            }
                        });
                    }
                }
                println!("Ожидание следующего интервала");
                sleep(collect_interval).await;
            }
        });
    });

    // Запускаем Tauri в основном потоке
    println!("Запуск Tauri");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            collect_perf_info_command,
            get_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
