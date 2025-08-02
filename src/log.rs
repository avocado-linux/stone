use simply_colored::*;

pub fn log_debug(message: &str) {
    println!("{WHITE}[DEBUG]{RESET} {message}");
}

pub fn log_info(message: &str) {
    println!("{BLUE}[INFO]{RESET} {message}");
}

// pub fn log_warning(message: &str) {
//     println!("{YELLOW}[WARNING]{RESET} {message}");
// }

pub fn log_success(message: &str) {
    println!("{GREEN}[SUCCESS]{RESET} {message}");
}

pub fn log_error(message: &str) {
    println!("{RED}[ERROR]{RESET} {message}");
}
