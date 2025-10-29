use mockito::Server;
use std::path::PathBuf;

pub fn setup_test_server() -> Server {
    Server::new()
}

pub fn get_test_data_path(file: &str) -> PathBuf {
    PathBuf::from("test_data").join(file)
}

pub fn read_test_json(file: &str) -> String {
    std::fs::read_to_string(get_test_data_path(file))
        .expect("Failed to read test data")
} 