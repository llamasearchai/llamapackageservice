use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressManager {
    show_progress: bool,
}

impl ProgressManager {
    pub fn new() -> Self {
        Self {
            show_progress: true,
        }
    }

    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .unwrap()
        );
        pb.set_message(message.to_string());
        pb
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}
