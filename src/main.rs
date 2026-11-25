use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = std::env::current_dir().expect("the current directory is readable");
    let code = prunr::cli::run(&args, &root, &mut std::io::stdout(), &mut std::io::stderr());
    ExitCode::from(code)
}
