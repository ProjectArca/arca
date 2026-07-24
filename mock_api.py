#!/usr/bin/env python3
"""
Mock OpenAI/Anthropic API Server - Complete AI API Testing Server
Serves exact OpenAI and Anthropic API format for testing Arca AI stdlib.

Supports:
- OpenAI Chat Completions (with streaming, function calling, JSON mode)
- OpenAI Embeddings
- Anthropic Messages API (Claude)
- Image Generation (DALL-E style)
- Speech to Text
- Text to Speech

Usage:
    python3 mock_api.py [--port 8080]
    MOCK_API_PORT=8080 python3 mock_api.py
"""

import json
import os
import random
import hashlib
import time
import base64
import wave
import struct
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

PORT = int(os.environ.get("MOCK_API_PORT", "8080"))

# ============================================================================
# Response Generators - OpenAI Format
# ============================================================================

def make_chat_response(model, messages, tools=None, stream=False, response_format=None):
    """Generate OpenAI-compatible chat completion response."""
    if stream:
        return None
    
    last_msg = messages[-1]["content"] if messages else "Hello"
    msg_hash = hashlib.md5(str(time.time()).encode()).hexdigest()[:8]
    
    # Handle function calling
    tool_calls = None
    finish_reason = "stop"
    
    if tools and any("function" in str(t).lower() for t in tools):
        # Simulate function call for certain keywords
        if any(kw in last_msg.lower() for kw in ["weather", "calculate", "search"]):
            tool_calls = [{
                "index": 0,
                "id": f"call_{msg_hash}",
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "arguments": json.dumps({"location": "Tokyo", "unit": "celsius"})
                }
            }]
            finish_reason = "tool_calls"
    
    # Handle JSON mode
    content = f"You said: {last_msg[:100]}... This is a mock response from the Arca AI test server."
    
    if response_format and response_format.get("type") == "json_object":
        content = json.dumps({
            "response": content,
            "status": "success",
            "timestamp": int(time.time())
        })
    
    response = {
        "id": f"chatcmpl-{msg_hash}",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
                "tool_calls": tool_calls
            },
            "finish_reason": finish_reason
        }],
        "usage": {
            "prompt_tokens": len(str(messages)),
            "completion_tokens": 20,
            "total_tokens": len(str(messages)) + 20
        }
    }
    
    # Add system_fingerprint for newer API versions
    response["system_fingerprint"] = f"fp_{msg_hash}"
    
    return response


def make_streaming_chunk(model, content, index=0, finish_reason=None):
    """Generate a single streaming chunk in OpenAI format."""
    msg_hash = hashlib.md5(str(time.time()).encode()).hexdigest()[:8]
    return {
        "id": f"chatcmpl-{msg_hash}",
        "object": "chat.completion.chunk",
        "created": int(time.time()),
        "model": model,
        "choices": [{
            "index": index,
            "delta": {"content": content},
            "finish_reason": finish_reason
        }]
    }


def make_embedding_response(input_text, model="text-embedding-ada-002"):
    """Generate OpenAI-compatible embeddings response."""
    # Use different dimensions based on model
    dimensions = 1536 if "ada" in model else 3072
    
    response = {
        "object": "list",
        "data": [],
        "model": model,
        "usage": {
            "prompt_tokens": 0,
            "total_tokens": 0
        }
    }
    
    texts = input_text if isinstance(input_text, list) else [input_text]
    
    for i, text in enumerate(texts):
        # Generate deterministic-ish embeddings based on text
        seed = hash(text) % (2**32)
        random.seed(seed)
        embedding = [round(random.uniform(-1, 1), 6) for _ in range(dimensions)]
        
        response["data"].append({
            "object": "embedding",
            "embedding": embedding,
            "index": i
        })
        response["usage"]["prompt_tokens"] += len(text.split())
    
    response["usage"]["total_tokens"] = response["usage"]["prompt_tokens"]
    return response


def make_image_response(prompt, model="dall-e-3", size="1024x1024", quality="standard", n=1):
    """Generate OpenAI-compatible image generation response."""
    response = {
        "created": int(time.time()),
        "data": []
    }
    
    for i in range(n):
        # Generate a placeholder base64 image
        placeholder = f"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        # In production, this would be actual generated image
        # For testing, we return a tiny 1x1 PNG
        
        response["data"].append({
            "url": f"https://mock.api/v1/images/generated/{hashlib.md5(str(time.time() + i).encode()).hexdigest()[:8]}.png",
            "b64_json": placeholder if "b64_json" in str(model) else None,
            "revised_prompt": f"{prompt} (artistic interpretation)"
        })
    
    return response


def make_stt_response(audio_data=None, model="whisper-1", language=None):
    """Generate OpenAI-compatible speech-to-text response."""
    return {
        "text": "This is a mock transcription of the audio content. Hello world from the Arca AI test server.",
        "language": language or "en",
        "duration": 3.5,
        "model": model
    }


def make_tts_response(text, model="tts-1", voice="alloy", response_format="mp3"):
    """Generate OpenAI-compatible text-to-speech response."""
    # Return a minimal valid audio file (silent MP3 header for testing)
    # In production, this would be actual TTS audio
    return {
        "audio_data": base64.b64encode(b"\x00\x00\x00\x1c\x66\x74\x79\x70\x69\x73\x6f\x6d").decode(),  # ftyp-isom
        "model": model,
        "voice": voice,
        "format": response_format
    }


# ============================================================================
# Response Generators - Anthropic Format
# ============================================================================

def make_anthropic_response(messages, model="claude-3-sonnet-20240229", system=None, stream=False):
    """Generate Anthropic-compatible Messages API response."""
    if stream:
        return None
    
    last_msg = [m for m in messages if m.get("role") == "user"][-1] if messages else {"content": "Hello"}
    content = last_msg.get("content", "Hello")
    msg_hash = hashlib.md5(str(time.time()).encode()).hexdigest()[:8]
    
    response = {
        "id": f"msg_{msg_hash}",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": f"You said: {content[:100] if isinstance(content, str) else 'Hello'}... This is a mock response from the Anthropic-compatible test server."
        }],
        "model": model,
        "stop_reason": "end_turn",
        "stop_sequence": None,
        "usage": {
            "input_tokens": 50,
            "output_tokens": 30
        }
    }
    
    return response


def make_anthropic_stream_chunk(content, index=0, type="content_block_delta"):
    """Generate Anthropic-compatible streaming chunk."""
    msg_hash = hashlib.md5(str(time.time()).encode()).hexdigest()[:8]
    return {
        "id": f"msg_{msg_hash}",
        "type": type,
        "index": index,
        "content_block": {
            "type": "text",
            "text": content
        } if type == "content_block_start" else {"delta": content}
    }


# ============================================================================
# Model Registry
# ============================================================================

MODELS = {
    # OpenAI Models
    "gpt-4": {"id": "gpt-4", "object": "model", "created": 1687882411, "owned_by": "openai", "permission": []},
    "gpt-4-turbo": {"id": "gpt-4-turbo", "object": "model", "created": 1712361441, "owned_by": "openai", "permission": []},
    "gpt-4o": {"id": "gpt-4o", "object": "model", "created": 1715361441, "owned_by": "openai", "permission": []},
    "gpt-4o-mini": {"id": "gpt-4o-mini", "object": "model", "created": 1720561441, "owned_by": "openai", "permission": []},
    "gpt-3.5-turbo": {"id": "gpt-3.5-turbo", "object": "model", "created": 1677610602, "owned_by": "openai", "permission": []},
    "text-embedding-ada-002": {"id": "text-embedding-ada-002", "object": "model", "created": 1671217299, "owned_by": "openai", "permission": []},
    "text-embedding-3-small": {"id": "text-embedding-3-small", "object": "model", "created": 1705948997, "owned_by": "openai", "permission": []},
    "text-embedding-3-large": {"id": "text-embedding-3-large", "object": "model", "created": 1705948998, "owned_by": "openai", "permission": []},
    "dall-e-3": {"id": "dall-e-3", "object": "model", "created": 1698816138, "owned_by": "openai", "permission": []},
    "dall-e-2": {"id": "dall-e-2", "object": "model", "created": 1677649963, "owned_by": "openai", "permission": []},
    "whisper-1": {"id": "whisper-1", "object": "model", "created": 1677532384, "owned_by": "openai", "permission": []},
    "tts-1": {"id": "tts-1", "object": "model", "created": 1699056516, "owned_by": "openai", "permission": []},
    # Anthropic Models
    "claude-3-opus-20240229": {"id": "claude-3-opus-20240229", "object": "model", "created": 1709591862, "owned_by": "anthropic", "permission": []},
    "claude-3-sonnet-20240229": {"id": "claude-3-sonnet-20240229", "object": "model", "created": 1709591861, "owned_by": "anthropic", "permission": []},
    "claude-3-5-sonnet-20241022": {"id": "claude-3-5-sonnet-20241022", "object": "model", "created": 1728561441, "owned_by": "anthropic", "permission": []},
    "claude-3-haiku-20240307": {"id": "claude-3-haiku-20240307", "object": "model", "created": 1710811148, "owned_by": "anthropic", "permission": []},
}

# ============================================================================
# HTTP Request Handler
# ============================================================================

class MockAPIHandler(BaseHTTPRequestHandler):
    """Handles all API endpoints."""
    
    def log_message(self, format, *args):
        print(f"[{self.log_date_time_string()}] {args[0]}")
    
    def send_json(self, data, status=200):
        """Send JSON response with proper headers."""
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'POST, GET, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization, Anthropic-Beta')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())
    
    def send_raw(self, data, content_type="application/json", status=200):
        """Send raw response."""
        self.send_response(status)
        self.send_header('Content-Type', content_type)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.end_headers()
        self.wfile.write(data)
    
    # -------------------------------------------------------------------
    # OpenAI Endpoints
    # -------------------------------------------------------------------
    
    def handle_chat_completions(self, data):
        """Handle OpenAI /v1/chat/completions endpoint."""
        model = data.get('model', 'gpt-3.5-turbo')
        messages = data.get('messages', [])
        stream = data.get('stream', False)
        tools = data.get('tools', None)
        response_format = data.get('response_format', None)
        temperature = data.get('temperature', 1.0)
        max_tokens = data.get('max_tokens', None)
        
        if stream:
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Cache-Control', 'no-cache')
            self.send_header('Connection', 'keep-alive')
            self.end_headers()
            
            last_msg = messages[-1]["content"] if messages else "Hello"
            words = last_msg[:200].split()
            
            for i, word in enumerate(words):
                chunk = make_streaming_chunk(
                    model, 
                    word + " ",
                    finish_reason=None
                )
                self.wfile.write(f"data: {json.dumps(chunk)}\n\n".encode())
                self.wfile.flush()
                time.sleep(0.05)
            
            # Send final chunk
            final_chunk = make_streaming_chunk(model, "", finish_reason="stop")
            self.wfile.write(f"data: {json.dumps(final_chunk)}\n\n".encode())
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
        else:
            response = make_chat_response(model, messages, tools, response_format=response_format)
            self.send_json(response)
    
    def handle_embeddings(self, data):
        """Handle OpenAI /v1/embeddings endpoint."""
        input_text = data.get('input', '')
        model = data.get('model', 'text-embedding-ada-002')
        encoding_format = data.get('encoding_format', 'float')
        
        if isinstance(input_text, list):
            response = {
                "object": "list",
                "data": [],
                "model": model,
                "usage": {"prompt_tokens": 0, "total_tokens": 0}
            }
            
            for i, text in enumerate(input_text):
                emb_response = make_embedding_response(text, model)
                emb_response["data"][0]["index"] = i
                response["data"].append(emb_response["data"][0])
                response["usage"]["prompt_tokens"] += emb_response["usage"]["prompt_tokens"]
            
            response["usage"]["total_tokens"] = response["usage"]["prompt_tokens"]
            self.send_json(response)
        else:
            response = make_embedding_response(input_text, model)
            self.send_json(response)
    
    def handle_images_generations(self, data):
        """Handle OpenAI /v1/images/generations endpoint."""
        prompt = data.get('prompt', '')
        model = data.get('model', 'dall-e-3')
        size = data.get('size', '1024x1024')
        quality = data.get('quality', 'standard')
        n = min(data.get('n', 1), 10)
        
        response = make_image_response(prompt, model, size, quality, n)
        self.send_json(response)
    
    def handle_audio_transcriptions(self, data, audio_data=None):
        """Handle OpenAI /v1/audio/transcriptions endpoint."""
        model = data.get('model', 'whisper-1')
        language = data.get('language', None)
        
        response = make_stt_response(audio_data, model, language)
        self.send_json(response)
    
    def handle_audio_speech(self, data):
        """Handle OpenAI /v1/audio/speech endpoint."""
        input_text = data.get('input', '')
        model = data.get('model', 'tts-1')
        voice = data.get('voice', 'alloy')
        response_format = data.get('response_format', 'mp3')
        
        # Return mock audio data
        response = make_tts_response(input_text, model, voice, response_format)
        audio_bytes = base64.b64decode(response["audio_data"])
        
        content_types = {
            "mp3": "audio/mpeg",
            "opus": "audio/opus",
            "aac": "audio/aac",
            "flac": "audio/flac"
        }
        
        self.send_raw(
            audio_bytes,
            content_type=content_types.get(response_format, "application/octet-stream")
        )
    
    # -------------------------------------------------------------------
    # Anthropic Endpoints  
    # -------------------------------------------------------------------
    
    def handle_anthropic_messages(self, data):
        """Handle Anthropic /v1/messages endpoint."""
        model = data.get('model', 'claude-3-sonnet-20240229')
        messages = data.get('messages', [])
        system = data.get('system', None)
        stream = data.get('stream', False)
        
        if stream:
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Cache-Control', 'no-cache')
            self.send_header('Connection', 'keep-alive')
            self.end_headers()
            
            last_msg = [m for m in messages if m.get("role") == "user"][-1] if messages else {"content": "Hello"}
            content = last_msg.get("content", "Hello")
            words = (content[:200] if isinstance(content, str) else "Hello").split()
            
            # Send content block start
            start_chunk = {
                "type": "content_block_start",
                "index": 0,
                "content_block": {"type": "text", "text": ""}
            }
            self.wfile.write(f"event: message_start\ndata: {json.dumps(start_chunk)}\n\n".encode())
            self.wfile.flush()
            
            for word in words:
                delta_chunk = {
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {"type": "text_delta", "text": word + " "}
                }
                self.wfile.write(f"event: content_block_delta\ndata: {json.dumps(delta_chunk)}\n\n".encode())
                self.wfile.flush()
                time.sleep(0.05)
            
            # Send message delta
            self.wfile.write(f"event: message_delta\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}}}}\\n\\n".encode())
            self.wfile.flush()
            self.wfile.write(b"event: message_stop\ndata: {}\n\n")
            self.wfile.flush()
        else:
            response = make_anthropic_response(messages, model, system)
            self.send_json(response)
    
    # -------------------------------------------------------------------
    # HTTP Methods
    # -------------------------------------------------------------------
    
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else '{}'
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        
        try:
            data = json.loads(body) if body else {}
            
            # OpenAI Endpoints
            if path in ['/v1/chat/completions', '/chat/completions']:
                self.handle_chat_completions(data)
            
            elif path in ['/v1/embeddings', '/embeddings']:
                self.handle_embeddings(data)
            
            elif path in ['/v1/images/generations', '/images/generations']:
                self.handle_images_generations(data)
            
            elif path in ['/v1/audio/transcriptions', '/audio/transcriptions']:
                self.handle_audio_transcriptions(data)
            
            elif path in ['/v1/audio/speech', '/audio/speech']:
                self.handle_audio_speech(data)
            
            # Anthropic Endpoints
            elif path in ['/v1/messages', '/v1/messages']:
                self.handle_anthropic_messages(data)
            
            else:
                self.send_json({
                    "error": {
                        "message": f"Endpoint not found: {path}",
                        "type": "invalid_request_error",
                        "code": "not_found"
                    }
                }, 404)
                
        except json.JSONDecodeError:
            self.send_json({
                "error": {
                    "message": "Invalid JSON in request body",
                    "type": "invalid_request_error",
                    "code": "parse_error"
                }
            }, 400)
        except Exception as e:
            self.send_json({
                "error": {
                    "message": f"Internal server error: {str(e)}",
                    "type": "internal_error",
                    "code": "server_error"
                }
            }, 500)
    
    def do_GET(self):
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        query = parse_qs(parsed_path.query)
        
        # OpenAI Endpoints
        if path in ['/v1/models', '/models']:
            response = {"object": "list", "data": list(MODELS.values())}
            self.send_json(response)
        
        elif path.startswith('/v1/models/'):
            model_id = path.split('/')[-1]
            if model_id in MODELS:
                self.send_json(MODELS[model_id])
            else:
                self.send_json({
                    "error": {
                        "message": f"Model not found: {model_id}",
                        "type": "invalid_request_error",
                        "code": "model_not_found"
                    }
                }, 404)
        
        # Anthropic Endpoints
        elif path == '/v1/organizations':
            self.send_json({
                "data": [{
                    "id": "org_mock",
                    "name": "Mock Organization",
                    "created_at": 1709591862
                }]
            })
        
        elif path == '/health':
            self.send_json({
                "status": "ok",
                "timestamp": int(time.time()),
                "version": "1.0.0"
            })
        
        elif path == '/':
            self.send_json({
                "name": "Arca AI Mock API Server",
                "version": "1.0.0",
                "endpoints": {
                    "openai": {
                        "chat": "POST /v1/chat/completions",
                        "embeddings": "POST /v1/embeddings",
                        "images": "POST /v1/images/generations",
                        "transcriptions": "POST /v1/audio/transcriptions",
                        "speech": "POST /v1/audio/speech",
                        "models": "GET /v1/models"
                    },
                    "anthropic": {
                        "messages": "POST /v1/messages",
                        "organizations": "GET /v1/organizations"
                    }
                }
            })
        
        else:
            self.send_json({
                "error": {
                    "message": f"Not found: {path}",
                    "type": "invalid_request_error",
                    "code": "not_found"
                }
            }, 404)
    
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'POST, GET, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type, Authorization, Anthropic-Beta')
        self.send_header('Access-Control-Max-Age', '86400')
        self.end_headers()


def main():
    """Start the mock API server."""
    server = HTTPServer(('0.0.0.0', PORT), MockAPIHandler)
    
    print(f"""
╔══════════════════════════════════════════════════════════════════╗
║                    Arca AI Mock API Server                        ║
╠══════════════════════════════════════════════════════════════════╣
║  URL:     http://localhost:{PORT}                                  ║
╠══════════════════════════════════════════════════════════════════╣
║  OpenAI Compatible Endpoints:                                      ║
║  ├─ POST /v1/chat/completions      - Chat completions             ║
║  ├─ POST /v1/embeddings           - Embeddings                    ║
║  ├─ POST /v1/images/generations   - Image generation             ║
║  ├─ POST /v1/audio/transcriptions - Speech to text               ║
║  ├─ POST /v1/audio/speech         - Text to speech               ║
║  └─ GET  /v1/models               - List models                   ║
╠══════════════════════════════════════════════════════════════════╣
║  Anthropic Compatible Endpoints:                                 ║
║  ├─ POST /v1/messages             - Claude messages               ║
║  └─ GET  /v1/organizations        - Organization info            ║
╠══════════════════════════════════════════════════════════════════╣
║  Utility Endpoints:                                              ║
║  ├─ GET  /health                  - Health check                 ║
║  └─ GET  /                        - API info                      ║
╚══════════════════════════════════════════════════════════════════╝
""")
    
    print(f"Server running on http://0.0.0.0:{PORT}")
    print("Press Ctrl+C to stop\n")
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nServer stopped")
        server.shutdown()


if __name__ == '__main__':
    main()
