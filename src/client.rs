mod structs;

use actix_web::web::JsonBody;
use text_io::read;
use std::fs;
use actix_web::{post, web, HttpRequest};
use serde::{Serialize, Deserialize};
use serde_json::json;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use reqwest::blocking::Client;
use std::process::Command;
pub use structs::judge_structs::*;

lazy_static! {
    static ref RES: Arc<Mutex<Option<Judge>>> = Arc::new(Mutex::new(None));
    static ref CLIENT: Client = Client::new();
}

fn submit(req: Submission) -> Result<Judge, reqwest::Error> {
    let response = CLIENT.post("http://127.0.0.1:12345/jobs")
        .json(&req)
        .send()
        .ok();
    let res = response.unwrap().json::<Judge>();
    print!("\x1B[2J\x1B[1;1H");
    println!("{:?}", res);
    res
}

fn main() {
    print!("\x1B[2J\x1B[1;1H");
    let mut req = Submission {source_code: "".to_string(), language: "".to_string(), user_id: 0, contest_id: 0, problem_id: 0};
    print!("Please input code path: ");
    let code_path: String = read!();
    req.source_code = fs::read_to_string(code_path).unwrap();
    print!("Please input language: ");
    req.language = read!();
    print!("Please input user id: ");
    req.user_id = read!();
    print!("Please input contest id: ");
    req.contest_id = read!();
    print!("Please input problem id: ");
    req.problem_id = read!();
    println!("{:?}", req);
    print!("\x1B[2J\x1B[1;1H");
    println!("Waiting...");
    submit(req);
}