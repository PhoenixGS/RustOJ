mod structs;
mod func;

use actix_web::{get, put, middleware::Logger, post, web, App, HttpServer, Responder, HttpRequest, HttpResponse, http::StatusCode};
use serde::{Serialize, Deserialize};
use env_logger;
use log;
//use core::lazy;
use std::{path::PathBuf, i64::MAX};
use structopt::StructOpt;
use std::{fs::{self, File}, io::{self, Write}, vec, mem::swap, cmp::Ordering};
use std::process::{Command, Stdio};
use wait_timeout::ChildExt;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use chrono::{Local, DateTime, FixedOffset, NaiveDate, prelude::*, offset::LocalResult};
pub use structs::{config_structs::*, judge_structs::*, user_structs::*, Errors};
pub use func::{gene_ret, get_TMPDIR, one_test};

//API

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    log::info!(target: "greet_handler", "Greeting {}", name);
    format!("Hello {name}!")
}

// DO NOT REMOVE: used in automatic testing
#[post("/internal/exit")]
#[allow(unreachable_code)]
async fn exit() -> impl Responder {
    log::info!("Shutdown as requested");
    std::process::exit(0);
    format!("Exited")
}

lazy_static! {
    static ref JUDGE: Arc<Mutex<Vec<Judge>>> = Arc::new(Mutex::new(Vec::new()));
    static ref USER: Arc<Mutex<Vec<User>>> = Arc::new(Mutex::new(Vec::new()));
}

fn judging(id: usize, config: &web::Data<Config>) -> Result<Judge, Errors> {
    let mut lock = JUDGE.lock().unwrap();
    if id >= lock.len() {
        return Err(Errors::ErrNotFound);
    }
    let judge = &mut lock[id];
    println!("start judging");
    
    //Get language
    let mut lang_c: Option<Language> = None;
    let mut lang: Language;
    for language in &config.languages {
        if judge.submission.language == language.name {
            lang_c = Some(language.clone());
        }
    }
    match lang_c {
        Some(la) => lang = la,
        None => return Err(Errors::ErrNotFound)
    }

    //Get problem
    let mut prob_c: Option<Problem> = None;
    let mut prob: Problem;
    for problem in &config.problems {
        if judge.submission.problem_id == problem.id {
            prob_c = Some(problem.clone());
        }
    }
    match prob_c {
        Some(pr) => prob = pr,
        None => return Err(Errors::ErrNotFound)
    }

    //Init judge
    judge.state = "Queueing".to_string();
    judge.result = "Waiting".to_string();
    judge.score = 0.0;
    judge.cases = vec![];
    judge.cases.push(CaseResult{id: 0, result: "Waiting".to_string(), time: 0, memory: 0, info: "".to_string()});
    let mut cnt = 0;
    for cas in &prob.cases
    {
        cnt += 1;
        judge.cases.push(CaseResult{id: cnt, result: "Waiting".to_string(), time: 0, memory: 0, info: "".to_string()});
    }

    let TMPDIR = get_TMPDIR();

    println!("mkdir -{}-", TMPDIR);

    Command::new("mkdir")
            .arg(TMPDIR.as_str())
            .output()
            .expect("fail to write code");

    let code_path = "./".to_string() + TMPDIR.as_str() + "/" + lang.file_name.as_str();
    let run_path = "./".to_string() + TMPDIR.as_str() + "/test";
    let mut file = File::create(code_path.as_str()).unwrap();
    file.write(judge.submission.source_code.as_bytes()).unwrap();

    //compile
    let mut com: String = "".to_string();
    let mut comm: Vec<String> = vec![];
    for st in &lang.command {
        if com.len() == 0 {
            com = st.clone();
        } else {
            if *st == "%OUTPUT%".to_string() {
                comm.push(run_path.clone());
            }
            else {
                if *st == "%INPUT%".to_string() {
                    comm.push(code_path.clone());
                } else {
                    comm.push(st.clone());
                }
            }
        }
    }

    println!("{}, {:?}", com, comm);

    let compile_res = Command::new(com).args(comm).status()?;
    println!("Com {:?}", compile_res);

    if ! compile_res.success() {
        judge.state = "Finished".to_string();
        judge.result = "Compilation Error".to_string();
        judge.cases[0].result = "Compilation Error".to_string();
        judge.score = 0.0;
        let now = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        judge.updated_time = now.clone();
        Command::new("rm")
                .arg("-rf")
                .arg(TMPDIR.as_str())
                .output()?;
        return Ok(judge.clone());
    }
    judge.cases[0].result = "Compilation Success".to_string();

    if prob.misc.is_some() && prob.misc.as_ref().unwrap().packing.is_some() {
        for pack in prob.misc.as_ref().unwrap().packing.as_ref().unwrap() {
            println!("{:?}", pack);
            let mut ff = true;
            let mut score_sum = 0.0;
            for case_id in pack {
                let index = *case_id as usize - 1;
                score_sum += prob.cases[index].score;
                match ff {
                    true => {
                        let res = one_test(&prob.cases[index], &run_path, &mut judge.cases[*case_id as usize], &prob.r#type);
                        println!("{:?}", res);
                        match res {
                            Ok(ref result) => {
                                if result.result != "Accepted".to_string() {
                                    ff = false;
                                    if judge.result == "Waiting".to_string() {
                                        judge.result = result.result.clone();
                                    }
                                }
                            },
                            Err(error) => return Err(error),
                        }
                        println!("{:?}", res);

                    }
                    false => judge.cases[*case_id as usize].result = "Skipped".to_string(),
                }
            }
            if ff == true {
                judge.score += score_sum;
            }
        }
    } else {
        let mut index:usize = 0;
        for cas in &prob.cases {
            println!("!!!{:?}", cas);
            index += 1;
            let res = one_test(cas, &run_path, &mut judge.cases[index], &prob.r#type);
            match res {
                Ok(result) => {
                    if result.result == "Accepted".to_string() {
                        judge.score += cas.score;
                    } else {
                        if judge.result == "Waiting".to_string() {
                            judge.result = result.result.clone();
                        }
                    }
                },
                Err(error) => return Err(error),
            }
        }
    }

    println!("{:?}", judge);

    let now = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    judge.updated_time = now.clone();
    if judge.result == "Waiting".to_string() {
        judge.result = "Accepted".to_string();
    }
    judge.state = "Finished".to_string();
    println!("{:?}", judge);
    Command::new("rm")
            .arg("-rf")
            .arg(TMPDIR.as_str())
            .output()?;
    Ok(judge.clone())
}

#[post("/jobs")]
async fn post_jobs(body: web::Json<Submission>, config: web::Data<Config>) -> impl Responder {
    log::info!("post_jobs");

    //Check language
    let mut lang_c: Option<Language> = None;
    for language in &config.languages {
        if body.language == language.name {
            lang_c = Some(language.clone());
        }
    }
    if lang_c.is_none() {
        return gene_ret(Err::<Judge, Errors>(Errors::ErrNotFound))
    }

    //Check user id
    let mut users = USER.lock().unwrap();
    if body.user_id >= users.len() as u64 {
        return gene_ret(Err::<Judge, Errors>(Errors::ErrNotFound));
    }
    drop(users);

    //todo: contest id

    //Check problem
    let mut prob_c: Option<Problem> = None;
    for problem in &config.problems {
        if body.problem_id == problem.id {
            prob_c = Some(problem.clone());
        }
    }
    if prob_c.is_none() {
        return gene_ret(Err::<Judge, Errors>(Errors::ErrNotFound))
    }
    
    //Init result
    let mut lock = JUDGE.lock().unwrap();
    println!("LEN!{}", lock.len());
    let now = Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let id = lock.len();
    let mut judge = Judge{id, created_time: now.clone(), updated_time: now.clone(), submission: body.clone(), state: "Queueing".to_string(), result: "Waiting".to_string(), score: 0.0, cases: vec![]};
    lock.push(judge.clone());
    drop(lock);

    //let res = judge(&body, lang.as_ref().unwrap(), prob.as_ref().unwrap());
    let res = judging(id, &config);
    gene_ret(res)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AskJob {
    user_id: Option<u64>,
    user_name: Option<String>,
    contest_id: Option<u64>,
    problem_id: Option<u64>,
    language: Option<String>,
    from: Option<String>,
    to: Option<String>,
    state: Option<String>,
    result: Option<String>,
}

#[get("/jobs")]
async fn get_jobs(info: web::Query<AskJob>, config: web::Data<Config>) -> impl Responder {
    let mut res: Vec<Judge> = vec![];
    let mut lock = JUDGE.lock().unwrap();
    let mut users = USER.lock().unwrap();
    for i in 0..lock.len() {
        let judge = &lock[i];
        let mut ff = true;
        if info.user_id.is_some() {
            if judge.submission.user_id != info.user_id.unwrap() {
                ff = false;
            }
        }
        if info.user_name.is_some() {
            if users[judge.submission.user_id as usize].name != *info.user_name.as_ref().unwrap() {
                ff = false;
            }
        }
        //todo: contest_id
        if info.problem_id.is_some() {
            if judge.submission.problem_id != info.problem_id.unwrap() {
                ff = false;
            }
        }
        if info.language.is_some() {
            if judge.submission.language != *info.language.as_ref().unwrap() {
                ff = false;
            }
        }
        if info.from.is_some() {
            if Local.datetime_from_str(judge.created_time.as_str(), "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap() < Local.datetime_from_str(info.from.as_ref().unwrap().as_str(), "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap() {
                ff = false;
            }
        }
        if info.to.is_some() {
            if Local.datetime_from_str(judge.created_time.as_str(), "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap() > Local.datetime_from_str(info.to.as_ref().unwrap().as_str(), "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap() {
                ff = false;
            }
        }
        if info.state.is_some() {
            if judge.state != *info.state.as_ref().unwrap() {
                ff = false;
            }
        }
        if info.result.is_some() {
            if judge.result != *info.result.as_ref().unwrap() {
                ff = false;
            }
        }
        if ff {
            res.push(lock[i].clone());
        }
    }
    println!("GET result: {:?}", res);
    HttpResponse::Ok().json(res)
}

#[get("/jobs/{index}")]
async fn get_jobs_id(index: web::Path<usize>, config: web::Data<Config>) -> impl Responder {
    let id = *index;
    let lock = JUDGE.lock().unwrap();
    let mut res: Result<Judge, Errors>;
    if id >= lock.len() {
        res = Err(Errors::ErrNotFound);
    } else {
        res = Ok(lock[id].clone());
    }
    gene_ret(res)
}

#[put("/jobs/{index}")]
async fn put_jobs_id(index: web::Path<usize>, config: web::Data<Config>) -> impl Responder {
    let id = *index;
    let res = judging(id, &config);
    gene_ret(res)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AddUser {
    id: Option<u64>,
    name: String,
}

#[post("/users")]
async fn post_users(body: web::Json<AddUser>, config: web::Data<Config>) -> impl Responder {
    let mut users = USER.lock().unwrap();
    if body.id.is_none() {
        let mut ff = true;
        for i in 0..users.len() {
            if users[i].name == body.name {
                ff = false;
            }
        }
        if ff == false {
            return gene_ret(Err::<User, Errors>(Errors::ErrInvalidArgument));
        }
        let user = User{id: users.len() as u64, name: body.name.clone()};
        users.push(user.clone());
        gene_ret(Ok(user))
    } else {
        let mut ff = true;
        for i in 0..users.len() {
            if users[i].name == body.name {
                ff = false;
            }
        }
        //Priority
        if body.id.unwrap() >= users.len() as u64 {
            return gene_ret(Err::<User, Errors>(Errors::ErrNotFound));
        }
        if ff == false {
            return gene_ret(Err::<User, Errors>(Errors::ErrInvalidArgument));
        }
        users[body.id.unwrap() as usize].name = body.name.clone();
        gene_ret(Ok(users[body.id.unwrap() as usize].clone()))
    }
}

#[get("/users")]
async fn get_users(config: web::Data<Config>) -> impl Responder {
    let mut users = USER.lock().unwrap();
    let mut res = users.clone();
    res.sort_by(|x, y| x.id.partial_cmp(&y.id).unwrap());
    gene_ret(Ok(res))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ScoringRule {
    latest,
    highest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum TieBreaker {
    submission_time,
    submission_count,
    user_id,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Rule {
    scoring_rule: Option<ScoringRule>,
    tie_breaker: Option<TieBreaker>,
}

#[get("/contests/{index}/ranklist")]
async fn ranklist(index: web::Path<usize>, info: web::Query<Rule>, config: web::Data<Config>) -> impl Responder {
    struct Rec {
        id: u64,
        time: DateTime<Local>,
        count: u64,
        score: f64,
    }

    impl Rec {
        fn cmp0(&self, other: &Rec, ty: &Option<TieBreaker>) -> Ordering {
            match self.score.partial_cmp(&other.score) {
                Some(Ordering::Equal) => {
                    match ty {
                        Some(TieBreaker::submission_time) => self.time.partial_cmp(&other.time).unwrap(),
                        Some(TieBreaker::submission_count) => self.count.partial_cmp(&other.count).unwrap(),
                        Some(TieBreaker::user_id) => self.id.partial_cmp(&other.id).unwrap(),
                        _ => Ordering::Equal,
                    }
                },
                _ => other.score.partial_cmp(&self.score).unwrap(),
            }
        }

        fn cmp(&self, other: &Rec, ty: &Option<TieBreaker>) -> Ordering {
            let t = Self::cmp0(&self, other, ty);
            match t {
                Ordering::Equal => self.id.partial_cmp(&other.id).unwrap(),
                _ => t
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    struct Ret {
        user: User,
        rank: u64,
        scores: Vec<f64>,
    }

    println!("Ranklist!");
    let rule;
    match &info.scoring_rule {
        Some(r) => rule = (*r).clone(),
        None => rule = ScoringRule::latest,
    }

    let mut users = USER.lock().unwrap();
    let mut vec:Vec<Rec> = vec![];
    for i in 0..users.len() {
        vec.push(Rec{id: i as u64, time: Local.ymd(9999, 12, 31).and_hms(23, 59, 59), count: 0, score: 0.0});
    }

    let mut ret = vec![];
    for i in 0..users.len() {
        ret.push(Ret{user: users[i].clone(), rank: 0, scores: vec![0.0; config.problems.len()]});
    }

    println!("Ranklist!");
    let lock = JUDGE.lock().unwrap();
    println!("Ranklist! {:?}", info.scoring_rule);
    let mut has_submitted = vec![];
    for i in 0..users.len() {
        has_submitted.push(vec![false; config.problems.len()]);
    }
    for i in 0..lock.len() {
        let id = lock[i].submission.user_id as usize;
        vec[id].count += 1;
        match &rule {
            ScoringRule::latest => {
                let time = Local.datetime_from_str(lock[i].created_time.as_str(), "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap();
                vec[id].time = time;
                ret[id].scores[lock[i].submission.problem_id as usize] = lock[i].score
            },
            ScoringRule::highest => {
                if has_submitted[id][lock[i].submission.problem_id as usize] == false || lock[i].score > ret[id].scores[lock[i].submission.problem_id as usize] {
                    let time = Local.datetime_from_str(lock[i].created_time.as_str(), "%Y-%m-%dT%H:%M:%S%.3fZ").unwrap();
                    vec[id].time = time;
                    ret[id].scores[lock[i].submission.problem_id as usize] = lock[i].score;
                    has_submitted[id][lock[i].submission.problem_id as usize] = true;
                }
            },
        }
    }

    println!("{:?}", ret);

    for i in 0..users.len() {
        for j in 0..config.problems.len() {
            vec[i].score += ret[i].scores[j];
        }
    }

    for i in 0..users.len() {
        println!("{}:{}", i, vec[i].score);
    }
    vec.sort_by(|x, y| x.cmp(y, &info.tie_breaker));

    ret[vec[0].id as usize].rank = 1;
    for i in 1..users.len() {
        if vec[i - 1].cmp0(&vec[i], &info.tie_breaker) != Ordering::Equal {
            ret[vec[i].id as usize].rank = i as u64 + 1;
        } else {
            ret[vec[i].id as usize].rank = ret[vec[i - 1].id as usize].rank;
        }
    }
    let mut result = vec![];
    for i in 0..users.len() {
        result.push(ret[vec[i].id as usize].clone());
    }
    gene_ret(Ok(result))
}

//Arguments
#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    //Set config
    #[structopt(short = "c", long = "config", default_value = "")]
    config: String,

    //Set flush
    #[structopt(short = "f", long = "flush-data")]
    flush: bool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();

    //Load config
    if opt.config == "".to_string() {
        panic!("Config Error");
    }
    let con_res = fs::read_to_string(opt.config.as_str())?;
    let config: Config = serde_json::from_str(con_res.as_str())?;

    println!("{:?}", config);
    //Set server
    let server: String;
    match config.server.bind_address
    {
        Some(ref serv) => server = serv.clone(),
        None => server = "127.0.0.1".to_string(),
    }
    let port: u16;
    match config.server.bind_port
    {
        Some(pt) => port = pt,
        None => port = 12345,
    }

    let mut users = USER.lock().unwrap();
    users.push(User{id: 0, name: "root".to_string()});
    drop(users);

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .wrap(Logger::default())
            .route("/hello", web::get().to(|| async { "Hello World!" }))
            .service(greet)
            .service(post_jobs)
            .service(get_jobs)
            .service(get_jobs_id)
            .service(put_jobs_id)
            .service(post_users)
            .service(get_users)
            .service(ranklist)
            // DO NOT REMOVE: used in automatic testing
            .service(exit)
    })
    .bind((server.as_str(), port))?
    .run()
    .await
}

