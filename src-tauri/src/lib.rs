use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use lazy_static::lazy_static;
use log::{error, info};
use reqwest::Client;
use serde::Serialize;
use simplelog::{CombinedLogger, Config, LevelFilter, WriteLogger};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
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
    pub static ref LOGGING_ENABLED: bool = env::var("LOGGING")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);
}

pub fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    if *LOGGING_ENABLED {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("perf_monitor.log")?;

        CombinedLogger::init(vec![WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            log_file,
        )])?;

        info!("Логирование инициализировано");
    }
    Ok(())
}

pub fn collect_perf_info() -> PerfInfo {
    let mut system = SYSTEM.lock().unwrap();
    if *LOGGING_ENABLED {
        info!("Получен доступ к SYSTEM");
    }

    // Обновляем данные
    system.refresh_all();
    system.refresh_cpu();
    if *LOGGING_ENABLED {
        info!("Системные данные обновлены");
    }

    let perf_info = PerfInfo {
        time: SystemTime::now(),
        system: SystemInfo {
            name: system.name().unwrap_or_default(),
            hostname: system.host_name().unwrap_or_default(),
        },
        cpu: system.cpus().iter().map(|cpu| cpu.cpu_usage()).collect(),
        memory: MemoryInfo {
            total: system.total_memory(),
            used: system.used_memory(),
            total_swap: system.total_swap(),
            used_swap: system.used_swap(),
        },
        processes: system
            .processes()
            .iter()
            .filter_map(|(pid, process)| {
                std::panic::catch_unwind(|| ProcessInfo {
                    pid: pid.as_u32() as i32,
                    name: process.name().to_string(),
                    cpu_usage: process.cpu_usage(),
                    memory: process.memory(),
                })
                .ok()
            })
            .collect(),
    };

    if *LOGGING_ENABLED {
        info!(
            "Собрана информация: {} процессов, {} ядер CPU",
            perf_info.processes.len(),
            perf_info.cpu.len()
        );
    }

    perf_info
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

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(json.as_bytes())
        .expect("Ошибка сжатия данных");
    let compressed_data = encoder.finish().expect("Ошибка финализации сжатия");

    if *LOGGING_ENABLED {
        info!(
            "Подготовлены данные для отправки: {} -> {} байт",
            json.len(),
            compressed_data.len()
        );
    }

    let server_url =
        env::var("SERVER_URL").unwrap_or_else(|_| "http://yourserver.com/api/monitor".to_string());

    let response = client
        .post(&server_url)
        .header("Content-Type", "application/json")
        .header("Content-Encoding", "gzip")
        .body(compressed_data)
        .send()
        .await?;

    if response.status().is_success() {
        let mut total = TOTAL_SENT.lock().unwrap();
        *total += batch.len();
        if *LOGGING_ENABLED {
            info!("Пакет успешно отправлен, всего: {}", *total);
        }
    } else if *LOGGING_ENABLED {
        error!("Ошибка отправки: {}", response.status());
    }

    Ok(())
}
