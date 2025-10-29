"""Screenshot capture utilities."""
import asyncio
import io
import platform
from typing import Optional, Tuple
import logging

from PIL import Image, ImageDraw
import pyautogui
import numpy as np


logger = logging.getLogger(__name__)


async def capture_screenshot_with_cursor() -> bytes:
    """Capture screenshot with cursor position marked."""
    # Get screenshot
    screenshot = pyautogui.screenshot()
    
    # Get cursor position
    cursor_x, cursor_y = pyautogui.position()
    
    # Draw cursor indicator
    draw = ImageDraw.Draw(screenshot)
    cursor_size = 20
    
    # Draw crosshair
    draw.line(
        [(cursor_x - cursor_size, cursor_y), (cursor_x + cursor_size, cursor_y)],
        fill=(255, 0, 0),
        width=2
    )
    draw.line(
        [(cursor_x, cursor_y - cursor_size), (cursor_x, cursor_y + cursor_size)],
        fill=(255, 0, 0),
        width=2
    )
    
    # Draw circle
    draw.ellipse(
        [
            (cursor_x - cursor_size // 2, cursor_y - cursor_size // 2),
            (cursor_x + cursor_size // 2, cursor_y + cursor_size // 2)
        ],
        outline=(255, 0, 0),
        width=2
    )
    
    # Convert to bytes
    output = io.BytesIO()
    screenshot.save(output, format='PNG')
    return output.getvalue()


async def capture_region(x: int, y: int, width: int, height: int) -> bytes:
    """Capture a specific region of the screen."""
    screenshot = pyautogui.screenshot(region=(x, y, width, height))
    
    output = io.BytesIO()
    screenshot.save(output, format='PNG')
    return output.getvalue()


def get_platform_screenshot_method():
    """Get platform-specific screenshot method."""
    system = platform.system()
    
    if system == "Darwin":  # macOS
        return capture_screenshot_macos
    elif system == "Windows":
        return capture_screenshot_windows
    elif system == "Linux":
        return capture_screenshot_linux
    else:
        return capture_screenshot_with_cursor


async def capture_screenshot_macos() -> bytes:
    """macOS-specific screenshot capture."""
    # Use screencapture command for better performance
    import subprocess
    import tempfile
    
    with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
        tmp_path = tmp.name
        
    try:
        # Capture screen
        subprocess.run(['screencapture', '-x', tmp_path], check=True)
        
        # Read and add cursor
        with Image.open(tmp_path) as img:
            cursor_x, cursor_y = pyautogui.position()
            
            draw = ImageDraw.Draw(img)
            cursor_size = 20
            
            # Draw cursor
            draw.ellipse(
                [
                    (cursor_x - cursor_size // 2, cursor_y - cursor_size // 2),
                    (cursor_x + cursor_size // 2, cursor_y + cursor_size // 2)
                ],
                outline=(255, 0, 0),
                width=2
            )
            
            output = io.BytesIO()
            img.save(output, format='PNG')
            return output.getvalue()
            
    finally:
        import os
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


async def capture_screenshot_windows() -> bytes:
    """Windows-specific screenshot capture."""
    # Use Windows API for better performance
    try:
        import win32gui
        import win32ui
        import win32con
        import win32api
        
        # Get screen dimensions
        hdesktop = win32gui.GetDesktopWindow()
        width = win32api.GetSystemMetrics(win32con.SM_CXVIRTUALSCREEN)
        height = win32api.GetSystemMetrics(win32con.SM_CYVIRTUALSCREEN)
        left = win32api.GetSystemMetrics(win32con.SM_XVIRTUALSCREEN)
        top = win32api.GetSystemMetrics(win32con.SM_YVIRTUALSCREEN)
        
        # Create device contexts
        desktop_dc = win32gui.GetWindowDC(hdesktop)
        img_dc = win32ui.CreateDCFromHandle(desktop_dc)
        mem_dc = img_dc.CreateCompatibleDC()
        
        # Create bitmap
        screenshot = win32ui.CreateBitmap()
        screenshot.CreateCompatibleBitmap(img_dc, width, height)
        mem_dc.SelectObject(screenshot)
        
        # Copy screen to bitmap
        mem_dc.BitBlt((0, 0), (width, height), img_dc, (left, top), win32con.SRCCOPY)
        
        # Convert to PIL Image
        bmpinfo = screenshot.GetInfo()
        bmpstr = screenshot.GetBitmapBits(True)
        img = Image.frombuffer(
            'RGB',
            (bmpinfo['bmWidth'], bmpinfo['bmHeight']),
            bmpstr, 'raw', 'BGRX', 0, 1
        )
        
        # Clean up
        mem_dc.DeleteDC()
        win32gui.DeleteObject(screenshot.GetHandle())
        
        # Add cursor
        cursor_x, cursor_y = pyautogui.position()
        draw = ImageDraw.Draw(img)
        cursor_size = 20
        
        draw.ellipse(
            [
                (cursor_x - cursor_size // 2, cursor_y - cursor_size // 2),
                (cursor_x + cursor_size // 2, cursor_y + cursor_size // 2)
            ],
            outline=(255, 0, 0),
            width=2
        )
        
        output = io.BytesIO()
        img.save(output, format='PNG')
        return output.getvalue()
        
    except ImportError:
        # Fallback to PyAutoGUI
        return await capture_screenshot_with_cursor()


async def capture_screenshot_linux() -> bytes:
    """Linux-specific screenshot capture."""
    # Try to use gnome-screenshot or scrot
    import subprocess
    import tempfile
    import shutil
    
    # Check available tools
    if shutil.which('gnome-screenshot'):
        tool = ['gnome-screenshot', '-f']
    elif shutil.which('scrot'):
        tool = ['scrot']
    else:
        # Fallback to PyAutoGUI
        return await capture_screenshot_with_cursor()
        
    with tempfile.NamedTemporaryFile(suffix='.png', delete=False) as tmp:
        tmp_path = tmp.name
        
    try:
        # Capture screen
        subprocess.run(tool + [tmp_path], check=True)
        
        # Read and add cursor
        with Image.open(tmp_path) as img:
            cursor_x, cursor_y = pyautogui.position()
            
            draw = ImageDraw.Draw(img)
            cursor_size = 20
            
            draw.ellipse(
                [
                    (cursor_x - cursor_size // 2, cursor_y - cursor_size // 2),
                    (cursor_x + cursor_size // 2, cursor_y + cursor_size // 2)
                ],
                outline=(255, 0, 0),
                width=2
            )
            
            output = io.BytesIO()
            img.save(output, format='PNG')
            return output.getvalue()
            
    except Exception:
        # Fallback
        return await capture_screenshot_with_cursor()
    finally:
        import os
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)