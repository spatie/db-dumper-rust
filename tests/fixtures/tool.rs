use std::{
    env,
    io::{self, Read, Write},
    thread,
    time::Duration,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = args
        .iter()
        .find_map(|s| s.strip_prefix("--fixture-mode="))
        .unwrap_or("dump");
    if mode == "fail-data" && args.iter().any(|s| s == "--no-create-info") {
        std::process::exit(8);
    }
    match mode {
        "fail" => {
            print!("partial dump");
            std::process::exit(7);
        }
        "empty" => {}
        "sleep" => thread::sleep(Duration::from_secs(5)),
        "noisy" => {
            io::stderr().write_all(&vec![b'!'; 512 * 1024]).unwrap();
            io::stdout().write_all(&vec![b'x'; 1024 * 1024]).unwrap();
        }
        "echo" => print!("{}", args.join("\0")),
        "copy" => {
            let mut input = vec![];
            io::stdin().read_to_end(&mut input).unwrap();
            io::stdout().write_all(&input).unwrap();
        }
        _ => print!(
            "/*M!999999\\- enable the sandbox mode */\nCREATE TABLE things (id INT);\n) ENGINE=InnoDB AUTO_INCREMENT=42;\nINSERT INTO things VALUES (' AUTO_INCREMENT=99');\n"
        ),
    }
}
