"""Action handler implementation."""
import asyncio
import pyautogui
import numpy as np
from typing import Optional, List, Tuple
import logging

from ..interfaces import (
    IActionInterface,
    IScreenInterface,
    Action,
    ActionType,
    Coordinate,
    OperationResult,
    OperationStatus
)
from ..utils.screenshot import capture_screenshot_with_cursor
from ..utils.ocr import OCRProcessor
from ..core.performance import PerformanceOptimizer


logger = logging.getLogger(__name__)

# Configure PyAutoGUI
pyautogui.FAILSAFE = True
pyautogui.PAUSE = 0.1


class ActionHandler(IActionInterface, IScreenInterface):
    """Handles action execution and screen operations."""
    
    def __init__(self, performance_optimizer: Optional[PerformanceOptimizer] = None):
        self.performance_optimizer = performance_optimizer
        self.ocr_processor = OCRProcessor()
        self._screen_width, self._screen_height = pyautogui.size()
        
    async def execute_action(self, action: Action) -> OperationResult:
        """Execute a single action."""
        try:
            if action.type == ActionType.CLICK:
                await self._execute_click(action)
            elif action.type == ActionType.TYPE:
                await self._execute_type(action)
            elif action.type == ActionType.KEY:
                await self._execute_key(action)
            elif action.type == ActionType.SCREENSHOT:
                result = await self.capture_screenshot()
                return OperationResult(
                    operation_id=action.id,
                    status=OperationStatus.SUCCESS,
                    result=result
                )
            elif action.type == ActionType.WAIT:
                await asyncio.sleep(float(action.value or 1))
            elif action.type == ActionType.SCROLL:
                await self._execute_scroll(action)
            elif action.type == ActionType.DRAG:
                await self._execute_drag(action)
            elif action.type == ActionType.HOVER:
                await self._execute_hover(action)
            else:
                raise ValueError(f"Unsupported action type: {action.type}")
                
            return OperationResult(
                operation_id=action.id,
                status=OperationStatus.SUCCESS
            )
            
        except Exception as e:
            logger.error(f"Action execution failed: {str(e)}")
            return OperationResult(
                operation_id=action.id,
                status=OperationStatus.FAILED,
                error=e
            )
            
    async def _execute_click(self, action: Action):
        """Execute click action."""
        if isinstance(action.target, Coordinate):
            x, y = action.target.x, action.target.y
        elif isinstance(action.target, str):
            # Try to find element by text
            coord = await self.find_element(action.target)
            if not coord:
                raise ValueError(f"Element not found: {action.target}")
            x, y = coord.x, coord.y
        else:
            # Assume percentage coordinates
            x_percent, y_percent = action.metadata.get("x", 50), action.metadata.get("y", 50)
            x = int(x_percent * self._screen_width / 100)
            y = int(y_percent * self._screen_height / 100)
            
        # Visual feedback
        await self._circular_mouse_movement(x, y)
        
        # Click
        button = action.metadata.get("button", "left")
        clicks = action.metadata.get("clicks", 1)
        pyautogui.click(x, y, button=button, clicks=clicks)
        
    async def _execute_type(self, action: Action):
        """Execute type action."""
        if not action.value:
            return
            
        # Type with optional interval
        interval = action.metadata.get("interval", 0.05)
        pyautogui.typewrite(str(action.value), interval=interval)
        
    async def _execute_key(self, action: Action):
        """Execute key press action."""
        if not action.value:
            return
            
        # Handle key combinations
        keys = action.value.split("+")
        if len(keys) > 1:
            pyautogui.hotkey(*keys)
        else:
            pyautogui.press(keys[0])
            
    async def _execute_scroll(self, action: Action):
        """Execute scroll action."""
        amount = int(action.value or 3)
        direction = action.metadata.get("direction", "down")
        
        if direction == "down":
            pyautogui.scroll(-amount)
        else:
            pyautogui.scroll(amount)
            
    async def _execute_drag(self, action: Action):
        """Execute drag action."""
        start = action.metadata.get("start")
        end = action.metadata.get("end")
        
        if not start or not end:
            raise ValueError("Drag requires start and end coordinates")
            
        duration = action.metadata.get("duration", 1.0)
        pyautogui.dragTo(end[0], end[1], duration=duration)
        
    async def _execute_hover(self, action: Action):
        """Execute hover action."""
        if isinstance(action.target, Coordinate):
            x, y = action.target.x, action.target.y
        else:
            x = action.metadata.get("x", 0)
            y = action.metadata.get("y", 0)
            
        pyautogui.moveTo(x, y)
        
    async def _circular_mouse_movement(self, target_x: int, target_y: int):
        """Move mouse in circular pattern before clicking."""
        current_x, current_y = pyautogui.position()
        
        # Move to target with smooth motion
        pyautogui.moveTo(target_x, target_y, duration=0.2)
        
        # Small circular motion for visual feedback
        radius = 5
        steps = 8
        for i in range(steps):
            angle = (i / steps) * 2 * 3.14159
            x = target_x + radius * np.cos(angle)
            y = target_y + radius * np.sin(angle)
            pyautogui.moveTo(x, y, duration=0.05)
            
        # Return to target
        pyautogui.moveTo(target_x, target_y, duration=0.1)
        
    async def validate_action(self, action: Action) -> bool:
        """Validate if an action can be performed."""
        if action.type == ActionType.CLICK:
            # Validate coordinates are within screen bounds
            if isinstance(action.target, Coordinate):
                return (0 <= action.target.x <= self._screen_width and 
                       0 <= action.target.y <= self._screen_height)
        return True
        
    async def get_supported_actions(self) -> List[ActionType]:
        """Get list of supported action types."""
        return list(ActionType)
        
    async def capture_screenshot(self) -> bytes:
        """Capture the current screen."""
        if self.performance_optimizer:
            return await self.performance_optimizer.optimize_screenshot_capture(
                capture_screenshot_with_cursor
            )
        return await capture_screenshot_with_cursor()
        
    async def get_screen_dimensions(self) -> Tuple[int, int]:
        """Get screen width and height."""
        return self._screen_width, self._screen_height
        
    async def find_element(self, identifier: str) -> Optional[Coordinate]:
        """Find an element on screen by identifier."""
        # Capture screenshot
        screenshot = await self.capture_screenshot()
        
        # Use OCR to find text
        ocr_results = await self.ocr_processor.extract_text(screenshot)
        
        for result in ocr_results:
            if identifier.lower() in result['text'].lower():
                # Return center of bounding box
                bbox = result['bbox']
                x = (bbox[0] + bbox[2]) // 2
                y = (bbox[1] + bbox[3]) // 2
                return Coordinate(x=x, y=y)
                
        return None