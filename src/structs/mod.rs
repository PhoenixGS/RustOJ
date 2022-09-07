pub mod config_structs {
    use serde::{Serialize, Deserialize};

    //Config
    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Server {
        pub bind_address: Option<String>,
        pub bind_port: Option<u16>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Misc {
        pub packing: Option<Vec<Vec<u64>>>,
        pub special_judge: Option<Vec<String>>,
        pub dynamic_ranking_ratio: Option<f64>,
        //
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Case {
        pub score: f64,
        pub input_file: String,
        pub answer_file: String,
        pub time_limit: u64,
        pub memory_limit: u64,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Problem {
        pub id: u64,
        pub name: String,
        pub r#type: String,
        pub misc: Misc,
        pub cases: Vec<Case>,
    }

    impl Problem {
        pub fn score_sum(&self) -> f64 {
            let mut res = 0.0;
            for case in &self.cases {
                res += case.score;
            }
            res
        }
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Language {
        pub name: String,
        pub file_name: String,
        pub command: Vec<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Config {
        pub server: Server,
        pub problems: Vec<Problem>,
        pub languages: Vec<Language>,
    }
    
    impl Config {
        pub fn to_index(&self, id: u64) -> Option<usize> {
            for i in 0..self.problems.len() {
                if id == self.problems[i].id {
                    return Some(i);
                }
            }
            None
        }
    }
}

pub mod judge_structs {
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Submission {
        pub source_code: String,
        pub language: String,
        pub user_id: u64,
        pub contest_id: u64,
        pub problem_id: u64,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct CaseResult {
        pub id: u64,
        pub result: String,
        pub time: u128,
        pub memory: u64,
        pub info: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Judge {
        pub id: usize,
        pub created_time: String,
        pub updated_time: String,
        pub submission: Submission,
        pub state: String,
        pub result: String,
        pub score: f64,
        pub cases: Vec<CaseResult>,
    }
}

pub mod user_structs {
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct User {
        pub id: u64,
        pub name: String,
    }
}

pub mod contest_structs {
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Contest {
        pub id: Option<u64>,
        pub name: String,
        pub from: String,
        pub to: String,
        pub problem_ids: Vec<u64>,
        pub user_ids: Vec<u64>,
        pub submission_limit: u64,
    }

    impl Contest {
        pub fn to_index(&self, id: u64) -> Option<usize> {
            for i in 0..self.problem_ids.len() {
                if id == self.problem_ids[i] {
                    return Some(i);
                }
            }
            None
        }
    }
}

#[derive(Debug)]
pub enum Errors {
    ErrInvalidArgument,
    ErrInvalidState,
    ErrNotFound,
    ErrRateLimit,
    ErrExternal,
    ErrInternal,
}

impl std::convert::From<std::io::Error> for Errors {
    fn from(err: std::io::Error) -> Self {
        Self::ErrInternal
    }
}

impl Errors {
    pub fn to_u16(&self) -> u16 {
        match self {
            Errors::ErrInvalidArgument => return 400,
            Errors::ErrInvalidState => return 400,
            Errors::ErrNotFound => return 404,
            Errors::ErrRateLimit => return 400,
            Errors::ErrExternal => return 500,
            Errors::ErrInternal => return 500,
        }
    }

    pub fn to_code(&self) -> u64 {
        match self {
            Errors::ErrInvalidArgument => return 1,
            Errors::ErrInvalidState => return 2,
            Errors::ErrNotFound => 3,
            Errors::ErrRateLimit => 4,
            Errors::ErrExternal => 5,
            Errors::ErrInternal => 6,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Errors::ErrInvalidArgument => return "ERR_INVALID_ARGUMENT".to_string(),
            Errors::ErrInvalidState => return "ERR_INVALID_STATE".to_string(),
            Errors::ErrNotFound => return "ERR_NOT_FOUND".to_string(),
            Errors::ErrRateLimit => return "ERR_RATE_LIMIT".to_string(),
            Errors::ErrExternal => return "ERR_EXTERNAL".to_string(),
            Errors::ErrInternal => return "ERR_INTERNAL".to_string(),
        }
    }
}