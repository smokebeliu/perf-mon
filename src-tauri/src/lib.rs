use lazy_static::lazy_static;
use reqwest::Client;
use serde::Serialize;
use std::env;
use std::sync::Mutex;
use std::time::SystemTime;
use sysinfo::{CpuExt, PidExt, ProcessExt, System, SystemExt};

#[derive(Serialize, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
}

#[derive(Serialize, Clone)]
pub struct SystemInfo {
    pub name: String,
    pub hostname: String,
}

#[derive(Serialize, Clone)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub total_swap: u64,
    pub used_swap: u64,
}

#[derive(Serialize, Clone)]
pub struct PerfInfo {
    pub time: SystemTime,
    pub system: SystemInfo,
    pub cpu: Vec<f32>,
    pub memory: MemoryInfo,
    pub processes: Vec<ProcessInfo>,
}

#[derive(Serialize)]
pub struct BufferStatus {
    pub last_item: Option<PerfInfo>,
    pub buffer_size: usize,
    pub total_sent: usize,
}

// Глобальный объект System для постоянного обновления данных
lazy_static! {
    pub static ref SYSTEM: Mutex<System> = Mutex::new(System::new_all());
    pub static ref DATA_BUFFER: Mutex<Vec<PerfInfo>> = Mutex::new(Vec::new());
    pub static ref TOTAL_SENT: Mutex<usize> = Mutex::new(0);
}

pub fn collect_perf_info() -> PerfInfo {
    let mut system = SYSTEM.lock().unwrap();
    println!("Получен доступ к SYSTEM");

    // Обновляем данные
    println!("Обновление системных данных");
    system.refresh_all();
    system.refresh_cpu();
    println!("Системные данные обновлены");

    // Получаем время
    let current_time = SystemTime::now();
    println!("Текущее время получено");

    // Получаем системную информацию
    println!("Начинаем сбор системной информации");
    let system_name = system.name().unwrap_or_else(|| {
        println!("Не удалось получить имя системы");
        String::from("Unknown")
    });
    let system_hostname = system.host_name().unwrap_or_else(|| {
        println!("Не удалось получить hostname");
        String::from("Unknown")
    });
    let system_info = SystemInfo {
        name: system_name,
        hostname: system_hostname,
    };
    println!("Системная информация собрана");

    // Получаем информацию о CPU
    println!("Начинаем сбор информации о CPU");
    let cpu_usage: Vec<f32> = system.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
    println!("Собрана информация о {} CPU ядрах", cpu_usage.len());

    // Получаем информацию о памяти
    println!("Начинаем сбор информации о памяти");
    let memory_info = MemoryInfo {
        total: system.total_memory(),
        used: system.used_memory(),
        total_swap: system.total_swap(),
        used_swap: system.used_swap(),
    };
    println!("Информация о памяти собрана");

    // Получаем информацию о процессах
    println!("Начинаем сбор информации о процессах");
    let processes: Vec<ProcessInfo> = system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let result = std::panic::catch_unwind(|| ProcessInfo {
                pid: pid.as_u32() as i32,
                name: process.name().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
            });

            match result {
                Ok(proc_info) => {
                    println!(
                        "Успешно обработан процесс: {} (PID: {})",
                        proc_info.name, proc_info.pid
                    );
                    Some(proc_info)
                }
                Err(_) => {
                    println!("Ошибка при обработке процесса с PID: {}", pid.as_u32());
                    None
                }
            }
        })
        .collect();
    println!("Собрана информация о {} процессах", processes.len());

    // Собираем финальную структуру
    println!("Формируем итоговую структуру");
    PerfInfo {
        time: current_time,
        system: system_info,
        cpu: cpu_usage,
        memory: memory_info,
        processes: processes,
    }
}

pub fn get_buffer_status() -> BufferStatus {
    let buffer = DATA_BUFFER.lock().unwrap();
    let total_sent = TOTAL_SENT.lock().unwrap();

    BufferStatus {
        last_item: buffer.last().cloned(),
        buffer_size: buffer.len(),
        total_sent: *total_sent,
    }
}

// Обновляем функцию send_batch чтобы она увеличивала счетчик отправленных элементов
pub async fn send_batch(batch: Vec<PerfInfo>) -> Result<(), reqwest::Error> {
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

    if response.status().is_success() {
        let mut total = TOTAL_SENT.lock().unwrap();
        *total += batch.len();
    }

    println!("Пакет отправлен, статус: {}", response.status());
    Ok(())
}
