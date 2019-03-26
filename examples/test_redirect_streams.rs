extern crate daemonize;

use daemonize::Daemonize;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let stdout = &args[1];
    let stderr = &args[2];

    let stdout = std::fs::File::create(stdout).unwrap();
    let stderr = std::fs::File::create(stderr).unwrap();

    Daemonize::new()
        .stdout(stdout)
        .stderr(stderr)
        .start()
        .unwrap();

    println!("stdout");
    println!("newline");
    eprintln!("stderr");
    eprintln!("newline");
}
