"""OCR utilities for text extraction."""
import asyncio
import io
from typing import Dict, List, Optional, Tuple
import logging

import easyocr
import numpy as np
from PIL import Image


logger = logging.getLogger(__name__)


class OCRProcessor:
    """Process images for text extraction."""
    
    def __init__(self, languages: List[str] = ['en']):
        self.languages = languages
        self._reader = None
        self._lock = asyncio.Lock()
        
    async def _get_reader(self):
        """Get or create OCR reader."""
        async with self._lock:
            if self._reader is None:
                # Run in thread pool to avoid blocking
                loop = asyncio.get_event_loop()
                self._reader = await loop.run_in_executor(
                    None,
                    lambda: easyocr.Reader(self.languages, gpu=False)
                )
            return self._reader
            
    async def extract_text(self, image_bytes: bytes) -> List[Dict[str, any]]:
        """Extract text from image."""
        try:
            # Convert bytes to PIL Image
            image = Image.open(io.BytesIO(image_bytes))
            
            # Convert to numpy array
            image_array = np.array(image)
            
            # Get reader
            reader = await self._get_reader()
            
            # Run OCR in thread pool
            loop = asyncio.get_event_loop()
            results = await loop.run_in_executor(
                None,
                lambda: reader.readtext(image_array)
            )
            
            # Format results
            formatted_results = []
            for bbox, text, confidence in results:
                # Convert bbox to simple format
                x_coords = [point[0] for point in bbox]
                y_coords = [point[1] for point in bbox]
                
                formatted_results.append({
                    'text': text,
                    'confidence': confidence,
                    'bbox': [
                        min(x_coords),
                        min(y_coords),
                        max(x_coords),
                        max(y_coords)
                    ],
                    'center': [
                        (min(x_coords) + max(x_coords)) // 2,
                        (min(y_coords) + max(y_coords)) // 2
                    ]
                })
                
            return formatted_results
            
        except Exception as e:
            logger.error(f"OCR extraction failed: {str(e)}")
            return []
            
    async def find_text(self, image_bytes: bytes, search_text: str) -> Optional[Dict[str, any]]:
        """Find specific text in image."""
        results = await self.extract_text(image_bytes)
        
        search_lower = search_text.lower()
        for result in results:
            if search_lower in result['text'].lower():
                return result
                
        return None
        
    async def extract_clickable_elements(self, image_bytes: bytes) -> List[Dict[str, any]]:
        """Extract likely clickable elements from image."""
        results = await self.extract_text(image_bytes)
        
        # Filter for likely clickable elements
        clickable_keywords = [
            'button', 'click', 'submit', 'save', 'cancel', 'ok', 'yes', 'no',
            'next', 'previous', 'back', 'continue', 'login', 'sign', 'download',
            'upload', 'select', 'choose', 'browse', 'search', 'find', 'go'
        ]
        
        clickable = []
        for result in results:
            text_lower = result['text'].lower()
            
            # Check if text contains clickable keywords
            is_clickable = any(keyword in text_lower for keyword in clickable_keywords)
            
            # Check if text is short (likely a button)
            if len(result['text']) <= 20:
                is_clickable = True
                
            # Check confidence
            if is_clickable and result['confidence'] > 0.7:
                clickable.append(result)
                
        return clickable
        
    async def group_text_regions(self, image_bytes: bytes) -> List[Dict[str, any]]:
        """Group nearby text into regions."""
        results = await self.extract_text(image_bytes)
        
        if not results:
            return []
            
        # Sort by vertical position
        results.sort(key=lambda x: x['center'][1])
        
        # Group nearby text
        groups = []
        current_group = [results[0]]
        
        for i in range(1, len(results)):
            prev = results[i-1]
            curr = results[i]
            
            # Check if vertically close
            vertical_distance = abs(curr['center'][1] - prev['center'][1])
            
            if vertical_distance < 50:  # Threshold for same line/group
                current_group.append(curr)
            else:
                groups.append(current_group)
                current_group = [curr]
                
        if current_group:
            groups.append(current_group)
            
        # Create group summaries
        group_summaries = []
        for group in groups:
            # Combine bounding boxes
            all_x = []
            all_y = []
            texts = []
            
            for item in group:
                bbox = item['bbox']
                all_x.extend([bbox[0], bbox[2]])
                all_y.extend([bbox[1], bbox[3]])
                texts.append(item['text'])
                
            group_summaries.append({
                'text': ' '.join(texts),
                'bbox': [min(all_x), min(all_y), max(all_x), max(all_y)],
                'center': [(min(all_x) + max(all_x)) // 2, (min(all_y) + max(all_y)) // 2],
                'items': len(group)
            })
            
        return group_summaries