pub const PACKAGE_CLASSIFICATION: &str = r#"
You are a package classifier. Analyze the following URL and respond with EXACTLY ONE of these types (no other text):
- github-repository
- github-organization
- pypi-package
- rust-crate

URL: {0}

Remember: Respond with ONLY the package type, nothing else. For example, if given a PyPI URL, respond only with: pypi-package"#;

pub const PACKAGE_SUMMARY: &str = r#"
LlamaSearch Package Analysis

Please analyze and summarize the following package contents:

{0}

Format your response with these sections:
📦 Package Overview
✨ Key Features
🔗 Dependencies
📄 Notable Files
"#; 