// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use sysinfo::{ProcessExt, System, SystemExt};
use serde::Serialize;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::env;
use dotenv::dotenv;
use sysinfo::PidExt;


#[derive(Serialize, Clone)]
struct ProcessInfo {
    pid: i32,
    name: String,
    cpu_usage: f32,
    memory: u64, // в килобайтах
}

// Глобальный объект System для постоянного обновления данных
lazy_static! {
    static ref SYSTEM: Mutex<System> = Mutex::new(System::new_all());
    // Глобальный буфер для накопления снимков данных (каждый снимок – вектор ProcessInfo)
    static ref DATA_BUFFER: Mutex<Vec<Vec<ProcessInfo>>> = Mutex::new(Vec::new());
}

/// Команда Tauri для получения текущей информации о процессах.
/// Эта функция будет вызываться из фронтенда для отладки.
#[tauri::command]
fn get_current_process_info() -> Vec<ProcessInfo> {
    // Берём данные из глобального объекта, который обновляется в фоне
    let system = SYSTEM.lock().unwrap();
    system.processes()
        .iter()
        .map(|(pid, process)| ProcessInfo {
            pid: pid.as_u32() as i32,
            name: process.name().to_string(),
            cpu_usage: process.cpu_usage(),
            memory: process.memory(),
        })
        .collect()
}

/// Функция для отправки накопленного пакета данных на сервер.
/// URL сервера берётся из переменной окружения SERVER_URL.
async fn send_batch(batch: Vec<Vec<ProcessInfo>>) -> Result<(), reqwest::Error> {
    let client = Client::new();
    let json = serde_json::to_string(&batch)
        .expect("Ошибка сериализации данных в JSON");

    // Пытаемся получить URL из переменной окружения
    let server_url = env::var("SERVER_URL")
        .unwrap_or_else(|_| "http://yourserver.com/api/monitor".to_string());

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .body(json)
        .send()
        .await?;
    println!("Пакет отправлен, статус: {}", response.status());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_current_process_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


#[tokio::main]
async fn main() {
    // Загружаем переменные окружения из .env файла (если он есть)
    dotenv().ok();

    // Интервал сбора – каждые 5 секунд
    let collect_interval = Duration::from_secs(5);

    run();

    // Первичное обновление, чтобы задать начальное состояние
    {
        let mut system = SYSTEM.lock().unwrap();
        system.refresh_all();
        system.refresh_cpu();
    }

   tokio::spawn(async {
       let collect_interval = Duration::from_secs(60);
       loop {
           {
               let mut system = SYSTEM.lock().unwrap();
               system.refresh_all();
               system.refresh_cpu();
           }
           // Собираем снимок текущих данных
           let snapshot = get_current_process_info();
           {
               let mut buffer = DATA_BUFFER.lock().unwrap();
               buffer.push(snapshot);
               // Если накопилось 30 или более снимков, извлекаем первые 30 и отправляем пакет
               if buffer.len() >= 30 {
                   let batch = buffer.drain(0..30).collect::<Vec<_>>();
                   // Отправляем пакет в отдельной задаче, чтобы не блокировать основной цикл
                   tokio::spawn(async move {
                       if let Err(e) = send_batch(batch).await {
                           eprintln!("Ошибка отправки пакета: {:?}", e);
                       }
                   });
               }
           }
           sleep(collect_interval).await;
       }
   });

}
