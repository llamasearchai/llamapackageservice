use colored::*;

pub struct Banner {
    text: String,
}

impl Banner {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn render(&self) -> String {
        let width = self.text.len() + 4;
        let border = "=".repeat(width);
        
        format!(
            "\n{}\n{}{}{}\n{}\n",
            border.bright_blue(),
            "| ".bright_blue(),
            self.text.bright_white().bold(),
            " |".bright_blue(),
            border.bright_blue()
        )
    }
}
