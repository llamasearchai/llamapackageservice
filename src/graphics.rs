use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tokio::time::sleep;
use std::io::Write;

pub async fn show_llama_thinking_animation() {
    let frames = ["[PROCESSING]", "[PROCESSING.]", "[PROCESSING..]", "[PROCESSING...]"];
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap()
    );
    
    for frame in frames.iter().cycle().take(20) {
        pb.set_message(frame.to_string());
        sleep(Duration::from_millis(200)).await;
    }
    pb.finish_and_clear();
}

pub async fn show_llama_testing_animation() {
    let frames = vec![
        "[TESTING] ",
        "[TESTING.] ",
        "[TESTING..] ",
        "[TESTING...] ",
    ];

    for _ in 0..2 {
        for frame in &frames {
            print!("\r{}", frame);
            std::io::stdout().flush().unwrap();
            sleep(Duration::from_millis(200)).await;
        }
    }
    println!();
}

pub async fn show_llama_success(message: &str) {
    let pb = ProgressBar::new_spinner();
    pb.set_message(format!("[SUCCESS] {}", message.green()));
    sleep(Duration::from_millis(500)).await;
    pb.finish_and_clear();
}

pub async fn show_llama_error(message: &str) {
    let pb = ProgressBar::new_spinner();
    pb.set_message(format!("[ERROR] {}", message.red()));
    sleep(Duration::from_millis(500)).await;
    pb.finish_and_clear();
}

pub fn print_welcome_banner() {
    println!("{}", r#"
         __      _                     
       /\ \__  /\ \__                
  _____\ \ ,_\/  \ ,_\   LlamaSearch    
_______\ \ \/____\\ \/___ Package Analyzer
         \ \__\   \ \__\                
    "#.blue());
} 