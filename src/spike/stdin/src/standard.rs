use std::io::BufRead;

pub fn start_standard_stdin() {
    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();

    loop {
        let mut input = String::new();
        match stdin_lock.read_line(&mut input) {
            Ok(len) => if len == 0 {
                println!("start_standard_stdin - EOF received.");
                return;
            } else {
                println!("start_standard_stdin - Received line: {}", input);
            },
            Err(error) => {
                println!("start_standard_stdin - error: {}", error);
                return;
            }
        }
    }
}
