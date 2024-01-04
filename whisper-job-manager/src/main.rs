use std::{path::PathBuf, sync::Arc, time::Duration};

use actix_web::{middleware, web, App, HttpServer};
use tokio::sync::Mutex;

use crate::{
    constants::TMP_DIR,
    routes::{
        cancel_job::cancel_job, get_all_statuses::get_all_statuses, get_job::get_job,
        get_status::get_status, new_job::new_job,
    },
};

mod config;
mod constants;
mod routes;
mod scheduler;

const DEFAULT_CONFIG_FILE: &'static str = "config.json";

// TODO should be config
const SCHEDULER_RUN_PERIOD_MILLIS: u64 = 1000 * 30;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    log::info!("Loaded args: {:?}", args);

    let mut config_path = PathBuf::new();
    // TODO better args handling + file validation
    if args.len() < 2 {
        // Just the app name
        config_path.push(DEFAULT_CONFIG_FILE);
    } else if args.len() < 3 {
        // App name + config file path
        config_path.push(&args[1])
    }

    let config = config::read_config(config_path);

    log::info!("Loaded config: {:?}", config);

    log::info!("Creating temporary file directory {:?}", TMP_DIR.as_path());
    std::fs::create_dir_all(TMP_DIR.as_path())?;

    let scheduler_instance = Arc::new(Mutex::new(scheduler::Scheduler::default()));
    let config = Arc::new(config);
    let config_data = web::Data::new(config.clone());
    let app_state = web::Data::new(scheduler_instance.clone());

    log::info!("Starting scheduler task...");

    let scheduler_instance_background_task = scheduler_instance.clone();
    actix_web::rt::spawn(async move {
        loop {
            actix_web::rt::time::sleep(Duration::from_millis(SCHEDULER_RUN_PERIOD_MILLIS)).await;
            let mut sch = scheduler_instance_background_task.lock().await;
            sch.run().await;
        }
    });

    log::info!("Starting server at {}:{}", config.host, config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(app_state.clone())
            .app_data(config_data.clone())
            .service(new_job)
            .service(cancel_job)
            .service(get_status)
            .service(get_job)
            .service(get_all_statuses)
    })
    .bind((config.host.clone(), config.port))?
    .run()
    .await
}
