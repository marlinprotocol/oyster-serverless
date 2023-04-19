use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::Serialize;
use serde_json::Value;
use std::process::Child;

use crate::serverless::delete_file;

#[derive(Serialize)]
pub struct JsonResponse {
    pub status: String,
    pub message: String,
    pub data: Option<Value>,
}

pub struct ResponseOptions {
    pub capnp_file_path: Option<String>,
    pub js_file_path: Option<String>,
    pub workerd_process: Option<Child>,
    pub data: Option<serde_json::Value>,
    pub message: String,
    pub status_code: StatusCode,
}

pub struct ResponseHandler {
    options: ResponseOptions,
}

impl ResponseHandler {
    pub fn new(options: ResponseOptions) -> Self {
        ResponseHandler { options }
    }

    fn delete_files(&self) {
        if let Some(ref js_file_path) = self.options.js_file_path {
            if let Err(e) = delete_file(js_file_path) {
                log::error!("Error deleting JS file: {}", e);
            }
        }

        if let Some(ref capnp_file_path) = self.options.capnp_file_path {
            if let Err(e) = delete_file(capnp_file_path) {
                log::error!("Error deleting configuration file: {}", e);
            }
        }
    }

    fn terminate_workerd_process(&mut self) {
        if let Some(ref mut workerd_process) = self.options.workerd_process {
            let kill_workerd_process = workerd_process.kill();
            match kill_workerd_process {
                Ok(_) => {
                    log::info!("Workerd process {} terminated.", workerd_process.id())
                }
                Err(_) => {
                    log::error!("Error terminating the process : {}", workerd_process.id())
                }
            }
        }
    }

    pub fn create_json_response(mut self) -> JsonResponse {
        self.delete_files();
        self.terminate_workerd_process();

        let status = if self.options.status_code == StatusCode::OK {
            "success".to_string()
        } else {
            "error".to_string()
        };

        JsonResponse {
            status,
            message: self.options.message,
            data: self.options.data,
        }
    }

    pub fn create_http_response(self) -> HttpResponse {
        let status_code = self.options.status_code;
        let json_resp = self.create_json_response();
        HttpResponse::build(status_code).json(json_resp)
    }
}

//Generate response
pub fn response(
    capnp_file_path: Option<&str>,
    js_file_path: Option<&str>,
    workerd_process: Option<std::process::Child>,
    data: Option<serde_json::Value>,
    message: &str,
    status_code: StatusCode,
) -> HttpResponse {
    let options = ResponseOptions {
        capnp_file_path: capnp_file_path.map(|s| s.to_string()),
        js_file_path: js_file_path.map(|s| s.to_string()),
        workerd_process,
        data,
        message: message.to_string(),
        status_code,
    };

    let response_handler = ResponseHandler::new(options);
    response_handler.create_http_response()
}
