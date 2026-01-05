"""
Tests for error module.
"""

import pytest

from llamapackageservice.error import (
    LlamaError,
    ProcessorError,
    IOError,
    HttpError,
    ValidationError,
    NetworkError,
    RateLimitError,
    ConfigError,
    CacheError,
)


class TestErrorHierarchy:
    """Tests for error class hierarchy."""
    
    def test_llama_error_is_exception(self):
        """Test that LlamaError inherits from Exception."""
        error = LlamaError("test")
        assert isinstance(error, Exception)
    
    def test_processor_error_inherits_from_llama_error(self):
        """Test that ProcessorError inherits from LlamaError."""
        error = ProcessorError("test")
        assert isinstance(error, LlamaError)
        assert isinstance(error, Exception)
    
    def test_all_errors_inherit_from_llama_error(self):
        """Test that all error types inherit from LlamaError."""
        error_classes = [
            ProcessorError,
            IOError,
            HttpError,
            ValidationError,
            NetworkError,
            RateLimitError,
            ConfigError,
            CacheError,
        ]
        
        for error_class in error_classes:
            error = error_class("test message")
            assert isinstance(error, LlamaError), f"{error_class.__name__} should inherit from LlamaError"


class TestHttpError:
    """Tests for HttpError."""
    
    def test_http_error_with_status(self):
        """Test HttpError with status code."""
        error = HttpError("Not Found", status_code=404)
        
        assert str(error) == "Not Found"
        assert error.status_code == 404
    
    def test_http_error_without_status(self):
        """Test HttpError without status code."""
        error = HttpError("Connection failed")
        
        assert str(error) == "Connection failed"
        assert error.status_code is None


class TestRateLimitError:
    """Tests for RateLimitError."""
    
    def test_rate_limit_error_with_retry_after(self):
        """Test RateLimitError with retry_after."""
        error = RateLimitError("Rate limited", retry_after=60)
        
        assert str(error) == "Rate limited"
        assert error.retry_after == 60
    
    def test_rate_limit_error_without_retry_after(self):
        """Test RateLimitError without retry_after."""
        error = RateLimitError("Rate limited")
        
        assert str(error) == "Rate limited"
        assert error.retry_after is None


class TestErrorMessages:
    """Tests for error messages."""
    
    def test_error_message_preserved(self):
        """Test that error messages are preserved."""
        message = "This is a test error message"
        error = LlamaError(message)
        
        assert str(error) == message
    
    def test_processor_error_message(self):
        """Test processor error message."""
        error = ProcessorError("Failed to process package")
        assert "Failed to process package" in str(error)
    
    def test_validation_error_message(self):
        """Test validation error message."""
        error = ValidationError("Invalid URL format")
        assert "Invalid URL format" in str(error)


class TestErrorCatching:
    """Tests for catching errors."""
    
    def test_catch_llama_error(self):
        """Test catching LlamaError catches all subtypes."""
        def raise_processor_error():
            raise ProcessorError("test")
        
        with pytest.raises(LlamaError):
            raise_processor_error()
    
    def test_catch_specific_error(self):
        """Test catching specific error type."""
        def raise_http_error():
            raise HttpError("test", status_code=500)
        
        with pytest.raises(HttpError) as exc_info:
            raise_http_error()
        
        assert exc_info.value.status_code == 500


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
