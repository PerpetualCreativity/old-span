#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub ignore: Vec<String>,
    pub passthrough: Vec<String>,
    pub pre_run: Vec<PreRun>,
    pub filters: Vec<Filter>,
    pub extra_args: Vec<String>,
    pub default_template: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct PreRun {
    pub command: String,
    pub files: Vec<String>,
    #[serde(default = "def_error_on")]
    pub error_on: String,
    #[serde(default = "def_replace")]
    pub replace: bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct Filter {
    pub path: std::path::PathBuf,
    pub files: Vec<String>,
}

fn def_error_on() -> String {
    "none".into()
}
fn def_replace() -> bool {
    true
}
