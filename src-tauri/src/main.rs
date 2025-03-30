// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotenv::dotenv;
use log::{error, info};
use perf_mon_lib::{collect_perf_info, send_batch, setup_logging, DATA_BUFFER};
use tokio::time::{sleep, Duration};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "PerfMonitorService";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub fn service_main(_arguments: Vec<String>) {
    if let Err(e) = run_service() {
        error!("Ошибка сервиса: {:?}", e);
    }
}

pub fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                info!("Получена команда остановки сервиса");
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // Создаём runtime для асинхронных операций
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        let collect_interval = Duration::from_secs(30);
        info!("Сервис мониторинга запущен");

        loop {
            let snapshot = collect_perf_info();

            {
                let mut buffer = DATA_BUFFER.lock().unwrap();
                buffer.push(snapshot);

                if buffer.len() >= 50 {
                    let batch = buffer.drain(0..50).collect::<Vec<_>>();
                    tokio::spawn(async move {
                        if let Err(e) = send_batch(batch).await {
                            error!("Ошибка отправки пакета: {:?}", e);
                        }
                    });
                }
            }

            sleep(collect_interval).await;
        }
    });

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    setup_logging()?;

    // Запускаем сервис
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}
