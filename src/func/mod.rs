use super::structs::{config_structs::*, judge_structs::*, Errors};
use actix_web::{get, put, middleware::Logger, post, web, App, HttpServer, Responder, HttpRequest, HttpResponse, http::StatusCode};
use serde::{Serialize, Deserialize};
use std::process::{Command, Stdio};
use std::fs::File;
use std::time::Duration;
use wait_timeout::ChildExt;
use rand::Rng;

pub fn gene_ret(res: Result<impl Serialize + std::fmt::Debug, Errors>) -> HttpResponse {
    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct Error {
        reason: String,
        code: u64,
    }

    match res {
        Ok(json) => {
            println!("Result: Ok {:?}", json);
            HttpResponse::Ok().json(json)
        },
        Err(Er) => {
            println!("Result: Err {}", Er.to_string());
            HttpResponse::BadRequest().status(StatusCode::from_u16(Er.to_u16()).unwrap()).json(Error{reason: Er.to_string(), code: Er.to_code()})
            //todo: error
        }
    }
}

pub fn get_TMPDIR() -> String {
    let mut rng = rand::thread_rng();
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const len: usize = 5;
    (0..len)
    .map(|_| {
        let idx = rng.gen_range(0..CHARSET.len());
        CHARSET[idx] as char
        })
        .collect()
}

pub fn one_test(case: &Case, run_path: &String, res: &mut CaseResult, typ: &String) -> Result<CaseResult, Errors> {
    let in_file = File::open(case.input_file.clone())?;
    let out_file = File::create(run_path.clone() + ".out")?;
    let mut child = Command::new(run_path.clone())
            .stdin(Stdio::from(in_file))
            .stdout(Stdio::from(out_file))
            .stderr(Stdio::null())
            .spawn().unwrap();

    //todo: Memory limit

    //Time limit
    let mut limit = case.time_limit;
    if limit == 0 {
        limit = std::u64::MAX;
    }
    let time_limit = Duration::from_micros(case.time_limit);
    let status_code = match child.wait_timeout(time_limit).unwrap() {
        Some(status) => {println!("Status {} {}", status.success(), status.code().unwrap());status.code()},
        None => {
            res.result = "Time Limit Exceeded".to_string();
            child.kill().unwrap();
            child.wait().unwrap().code();
            return Ok(res.clone());
        }
    };
    if status_code.unwrap() != 0 {
        res.result = "Runtime Error".to_string();
        return Ok(res.clone());
    }
    
    let mut ret;
    if *typ == "standard".to_string() {
        ret = Command::new("diff")
                        .arg("-b")
                        .arg(case.answer_file.clone())
                        .arg(run_path.clone() + ".out")
                        .status().unwrap();
        //?
    } else {
        ret = Command::new("diff")
                        .arg(case.answer_file.clone())
                        .arg(run_path.clone() + ".out")
                        .status().unwrap()
    }
    if ret.success() {
        res.result = "Accepted".to_string();
    } else {
        res.result = "Wrong Answer".to_string();
    }
    Command::new("rm")
            .arg(run_path.clone() + ".out").output()?;
    Ok(res.clone())
}