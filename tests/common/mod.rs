use std::path::PathBuf;
use mockito::Server;
use llamapackageservice::config::Config;

pub mod test_helpers {
    use super::*;

    pub async fn setup_test_server() -> mockito::ServerGuard {
        let server = Server::new_async().await;
        // Point the GitHub API base to this mock server for tests
        std::env::set_var("GITHUB_API_BASE_URL", server.url());
        server
    }

    pub fn get_test_data_path(file: &str) -> PathBuf {
        PathBuf::from("test_data").join(file)
    }

    pub fn read_test_json(file: &str) -> String {
        std::fs::read_to_string(get_test_data_path(file))
            .expect("Failed to read test data")
    }

    pub fn create_test_config() -> Config {
        Config::default()
    }

    pub fn setup_test_logger() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    }
} 