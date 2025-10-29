use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::browser::state::{Mode, SharedState};
use crate::error::Result;

pub struct InputHandler;

impl InputHandler {
    pub async fn handle_global_keys(key: KeyEvent, state: &SharedState) -> Result<bool> {
        let mut state = state.write().await;
        
        match (key.modifiers, key.code) {
            // Mode switching
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if matches!(state.mode, Mode::Insert | Mode::Visual | Mode::Command) {
                    state.mode = Mode::Normal;
                    return Ok(false);
                }
            }
            
            // Command mode
            (KeyModifiers::NONE, KeyCode::Char(':')) if matches!(state.mode, Mode::Normal) => {
                state.mode = Mode::Command;
                state.command_buffer = ":".to_string();
                return Ok(false);
            }
            
            // Visual mode
            (KeyModifiers::NONE, KeyCode::Char('v')) if matches!(state.mode, Mode::Normal) => {
                state.mode = Mode::Visual;
                return Ok(false);
            }
            
            _ => {}
        }
        
        Ok(false)
    }
    
    pub async fn handle_command_mode(key: KeyEvent, state: &SharedState) -> Result<bool> {
        let mut state = state.write().await;
        
        if !matches!(state.mode, Mode::Command) {
            return Ok(false);
        }
        
        match key.code {
            KeyCode::Enter => {
                let command = state.command_buffer.clone();
                state.add_command_to_history(command.clone());
                state.mode = Mode::Normal;
                state.command_buffer.clear();
                
                // Process command
                return Self::process_command(&command[1..]).await;
            }
            
            KeyCode::Esc => {
                state.mode = Mode::Normal;
                state.command_buffer.clear();
            }
            
            KeyCode::Backspace => {
                if state.command_buffer.len() > 1 {
                    state.command_buffer.pop();
                }
            }
            
            KeyCode::Char(c) => {
                state.command_buffer.push(c);
            }
            
            KeyCode::Up => {
                if let Some(cmd) = state.get_previous_command() {
                    state.command_buffer = format!(":{}", cmd);
                }
            }
            
            KeyCode::Down => {
                if let Some(cmd) = state.get_next_command() {
                    state.command_buffer = format!(":{}", cmd);
                }
            }
            
            _ => {}
        }
        
        Ok(false)
    }
    
    async fn process_command(command: &str) -> Result<bool> {
        match command {
            "q" | "quit" => Ok(true),
            "w" | "write" => {
                // Save current state
                Ok(false)
            }
            "wq" => {
                // Save and quit
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}