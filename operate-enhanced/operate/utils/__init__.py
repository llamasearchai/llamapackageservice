"""Utility modules for operate framework."""
from .screenshot import capture_screenshot_with_cursor, capture_region
from .ocr import OCRProcessor

__all__ = [
    "capture_screenshot_with_cursor",
    "capture_region", 
    "OCRProcessor",
]