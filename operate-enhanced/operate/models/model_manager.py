"""Model manager for AI integrations."""
import base64
import json
import os
from typing import Any, Dict, List, Optional
import logging

import openai
import anthropic
import google.generativeai as genai
import httpx

from ..interfaces import Action, ActionType, IModelInterface


logger = logging.getLogger(__name__)


class ModelManager(IModelInterface):
    """Manages AI model interactions."""
    
    def __init__(self, model_name: str = "gpt-4o"):
        self.model_name = model_name
        self.clients = self._initialize_clients()
        self.system_prompt = self._load_system_prompt()
        
    def _initialize_clients(self) -> Dict[str, Any]:
        """Initialize AI clients."""
        clients = {}
        
        # OpenAI
        if os.getenv("OPENAI_API_KEY"):
            clients["openai"] = openai.OpenAI()
            
        # Anthropic
        if os.getenv("ANTHROPIC_API_KEY"):
            clients["anthropic"] = anthropic.Anthropic()
            
        # Google
        if os.getenv("GOOGLE_API_KEY"):
            genai.configure(api_key=os.getenv("GOOGLE_API_KEY"))
            clients["google"] = genai
            
        return clients
        
    def _load_system_prompt(self) -> str:
        """Load system prompt for the model."""
        return """You are an AI assistant helping to operate a computer. 
        Analyze the screenshot and determine the next action to achieve the objective.
        
        Available actions:
        - click: Click on an element (provide x,y coordinates as percentages)
        - type: Type text
        - key: Press keyboard keys (e.g., "Return", "Tab", "cmd+c")
        - scroll: Scroll up or down
        - wait: Wait for a specified time or indicate "done" when objective is complete
        
        Respond in JSON format:
        {
            "action": "click|type|key|scroll|wait",
            "coordinate": [x_percentage, y_percentage],  // for click actions
            "text": "text to type",  // for type actions
            "key": "key to press",  // for key actions
            "direction": "up|down",  // for scroll actions
            "value": "done|number",  // for wait actions
            "reasoning": "explanation of why this action"
        }
        """
        
    async def analyze_screen(self, screenshot: bytes, objective: str) -> Action:
        """Analyze screenshot and determine next action."""
        # Encode screenshot
        screenshot_b64 = base64.b64encode(screenshot).decode()
        
        if self.model_name.startswith("gpt"):
            return await self._analyze_with_openai(screenshot_b64, objective)
        elif self.model_name.startswith("claude"):
            return await self._analyze_with_anthropic(screenshot_b64, objective)
        elif self.model_name.startswith("gemini"):
            return await self._analyze_with_google(screenshot_b64, objective)
        else:
            raise ValueError(f"Unsupported model: {self.model_name}")
            
    async def _analyze_with_openai(self, screenshot_b64: str, objective: str) -> Action:
        """Analyze with OpenAI models."""
        client = self.clients.get("openai")
        if not client:
            raise ValueError("OpenAI client not initialized")
            
        response = client.chat.completions.create(
            model=self.model_name,
            messages=[
                {"role": "system", "content": self.system_prompt},
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": f"Objective: {objective}"},
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": f"data:image/png;base64,{screenshot_b64}"
                            }
                        }
                    ]
                }
            ],
            response_format={"type": "json_object"},
            temperature=0.1,
            max_tokens=300
        )
        
        return self._parse_response(response.choices[0].message.content)
        
    async def _analyze_with_anthropic(self, screenshot_b64: str, objective: str) -> Action:
        """Analyze with Anthropic models."""
        client = self.clients.get("anthropic")
        if not client:
            raise ValueError("Anthropic client not initialized")
            
        response = client.messages.create(
            model=self.model_name,
            messages=[
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": f"{self.system_prompt}\n\nObjective: {objective}"},
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": screenshot_b64
                            }
                        }
                    ]
                }
            ],
            max_tokens=300,
            temperature=0.1
        )
        
        # Extract JSON from response
        content = response.content[0].text
        json_start = content.find("{")
        json_end = content.rfind("}") + 1
        json_str = content[json_start:json_end]
        
        return self._parse_response(json_str)
        
    async def _analyze_with_google(self, screenshot_b64: str, objective: str) -> Action:
        """Analyze with Google models."""
        genai_client = self.clients.get("google")
        if not genai_client:
            raise ValueError("Google client not initialized")
            
        model = genai_client.GenerativeModel(self.model_name)
        
        # Decode image for Gemini
        import base64
        from PIL import Image
        import io
        
        image_data = base64.b64decode(screenshot_b64)
        image = Image.open(io.BytesIO(image_data))
        
        response = model.generate_content([
            f"{self.system_prompt}\n\nObjective: {objective}",
            image
        ])
        
        # Extract JSON from response
        content = response.text
        json_start = content.find("{")
        json_end = content.rfind("}") + 1
        json_str = content[json_start:json_end]
        
        return self._parse_response(json_str)
        
    def _parse_response(self, response_text: str) -> Action:
        """Parse model response into Action."""
        try:
            data = json.loads(response_text)
            
            # Map response to action
            action_type = ActionType(data["action"])
            
            # Build action based on type
            if action_type == ActionType.CLICK:
                coordinate = data.get("coordinate", [50, 50])
                from ..interfaces import Coordinate
                target = Coordinate.from_percentage(
                    coordinate[0], coordinate[1],
                    1920, 1080  # Default screen size, should be dynamic
                )
                value = None
            elif action_type == ActionType.TYPE:
                target = None
                value = data.get("text", "")
            elif action_type == ActionType.KEY:
                target = None
                value = data.get("key", "")
            elif action_type == ActionType.SCROLL:
                target = None
                value = data.get("direction", "down")
            elif action_type == ActionType.WAIT:
                target = None
                value = data.get("value", "1")
            else:
                target = None
                value = None
                
            return Action(
                id=f"model_{self.model_name}",
                type=action_type,
                target=target,
                value=value,
                metadata={"reasoning": data.get("reasoning", "")}
            )
            
        except Exception as e:
            logger.error(f"Failed to parse model response: {str(e)}")
            # Default to wait action on error
            return Action(
                id="error_fallback",
                type=ActionType.WAIT,
                value="1",
                metadata={"error": str(e)}
            )
            
    async def decompose_task(self, task: str) -> List[str]:
        """Break down a complex task into subtasks."""
        prompt = f"""Break down this task into smaller, actionable subtasks:
        
        Task: {task}
        
        Provide a numbered list of subtasks that can be executed sequentially.
        Keep each subtask focused and specific.
        """
        
        if "openai" in self.clients:
            client = self.clients["openai"]
            response = client.chat.completions.create(
                model="gpt-4",
                messages=[
                    {"role": "system", "content": "You are a task planning assistant."},
                    {"role": "user", "content": prompt}
                ],
                temperature=0.3,
                max_tokens=500
            )
            
            # Parse subtasks from response
            content = response.choices[0].message.content
            subtasks = []
            
            for line in content.split("\n"):
                line = line.strip()
                if line and (line[0].isdigit() or line.startswith("-")):
                    # Remove numbering and bullets
                    task_text = line.lstrip("0123456789.-) ").strip()
                    if task_text:
                        subtasks.append(task_text)
                        
            return subtasks
            
        return [task]  # Fallback to original task
        
    async def validate_result(self, expected: str, actual: Any) -> bool:
        """Validate if result matches expectation."""
        # Simple validation for now
        if isinstance(actual, str):
            return expected.lower() in actual.lower()
        return False