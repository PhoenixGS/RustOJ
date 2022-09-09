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
pub use structs::{judge_structs::*, user_structs::User, Errors};

lazy_static! {
    static ref RES: Arc<Mutex<Option<Judge>>> = Arc::new(Mutex::new(None));
    static ref CLIENT: Client = Client::new();
}

fn submit(req: Submission) -> Result<Judge, reqwest::Error> {
    let response = CLIENT.post("http://127.0.0.1:12345/jobs")
        .json(&req)
        .send()
        .ok();
    let res = response.unwrap().json::<Judge>()?;
//    print!("\x1B[2J\x1B[1;1H");
//    println!("{:?}", res);
    Ok(res)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Ret {
    user: User,
    rank: u64,
    scores: Vec<f64>,
}

fn query(contest_id: u64) -> Result<Vec<Ret>, reqwest::Error> {
    let response = CLIENT.get("http://127.0.0.1:12345/contests/".to_string() + contest_id.to_string().as_str() + "/ranklist")
        .send()
        .ok();
    let res = response.unwrap().json::<Vec<Ret>>()?;
    print!("\x1B[2J\x1B[1;1H");
    print!("Rank | User ID |");
    for i in 0..res[0].scores.len() {
        print!(" Problem {:>10} |", i);
    }
    println!("");
    for i in 0..res.len() {
        print!("{:<4} | {:>7} |", res[i].rank, res[i].user.name);
        for j in 0..res[i].scores.len() {
            print!(" {:>18} |", res[i].scores[j]);
        }
        println!("");
    }

    Ok(res)
}

fn print(judge: Judge) {

}

fn main() {
    print!("\x1B[2J\x1B[1;1H");
    while true {
        print!("What do you want to do? (1.Submit a code 2.Submit codes from many users 3.Query ranklist 4.Exit): ");
        let op: u64 = read!();
        match op {
            1 => {
                let mut req = Submission {source_code: "".to_string(), language: "".to_string(), user_id: 0, contest_id: 0, problem_id: 0};
                print!("\x1B[2J\x1B[1;1H");
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
                let res = submit(req);
                print!("\x1B[2J\x1B[1;1H");
                match res {
                    Ok(judge) => print(judge),
                    Err(err) => println!("Error: {:?}", err),
                }
            },
            2 => {
                print!("\x1B[2J\x1B[1;1H");
                print!("Please input json path: ");
                let json_path: String = read!();
                let st = fs::read_to_string(json_path).unwrap();
                let json: Vec<Submission> = serde_json::from_str(st.as_str()).unwrap();
                let mut results = vec![];
                for submission in json {
                    let res = submit(submission);
                    match res {
                        Ok(result) => results.push(result),
                        Err(err) => (),
                    }
                }
                fs::write("result.json", json!(results).to_string()).unwrap();
                println!("Please open result.json to get the result.");
            },
            3 => {
                print!("\x1B[2J\x1B[1;1H");
                print!("Please input contest id: ");
                let contest_id: u64 = read!();
                print!("\x1B[2J\x1B[1;1H");
                println!("Waiting...");
                query(contest_id);
            },
            4 => return,
            _ => (),
        }
    }
}