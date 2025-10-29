"""Setup script for operate-enhanced."""
from setuptools import setup, find_packages

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="operate-enhanced",
    version="2.0.0",
    author="Enhanced Team",
    description="Enhanced Self-Operating Computer Framework with GitHub integration",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/yourusername/operate-enhanced",
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
    ],
    python_requires=">=3.9",
    install_requires=[
        "pyautogui>=0.9.54",
        "opencv-python>=4.8.0",
        "pillow>=10.0.0",
        "openai>=1.0.0",
        "anthropic>=0.8.0",
        "google-generativeai>=0.3.0",
        "easyocr>=1.7.0",
        "pyyaml>=6.0",
        "pydantic>=2.5.0",
        "fastapi>=0.104.0",
        "uvicorn>=0.24.0",
        "websockets>=12.0",
        "pygithub>=2.1.0",
        "gitpython>=3.1.40",
        "redis>=5.0.0",
        "aiofiles>=23.2.0",
        "httpx>=0.25.0",
        "rich>=13.7.0",
        "click>=8.1.7",
        "numpy>=1.24.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.4.0",
            "pytest-asyncio>=0.21.0",
            "black>=23.12.0",
            "ruff>=0.1.0",
            "mypy>=1.7.0",
            "pytest-cov>=4.1.0",
        ]
    },
    entry_points={
        "console_scripts": [
            "operate=operate.main:cli",
        ],
    },
)