//! Standard Library compiler definitions and intrinsic mappings in Rust (`arca-stdlib`).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StdSymbol {
    pub name: String,
    pub module: String,
    pub is_intrinsic: bool,
}

pub struct StdLibResolver {
    symbols: HashMap<String, Vec<StdSymbol>>,
}

impl StdLibResolver {
    pub fn new() -> Self {
        let mut symbols = HashMap::new();

        // Core module symbols (zero OS, zero allocator)
        let core_syms = vec![
            StdSymbol { name: "mem".into(), module: "core".into(), is_intrinsic: true },
            StdSymbol { name: "math".into(), module: "core".into(), is_intrinsic: true },
            StdSymbol { name: "hash".into(), module: "core".into(), is_intrinsic: false },
            StdSymbol { name: "encoding".into(), module: "core".into(), is_intrinsic: false },
        ];
        symbols.insert("core".into(), core_syms);

        // Std module symbols (OS, Network, I/O, Allocators)
        let std_alloc = vec![StdSymbol { name: "ArenaAllocator".into(), module: "std/alloc".into(), is_intrinsic: false }];
        let std_fs = vec![
            StdSymbol { name: "File".into(), module: "std/fs".into(), is_intrinsic: false },
            StdSymbol { name: "Directory".into(), module: "std/fs".into(), is_intrinsic: false },
            StdSymbol { name: "exists".into(), module: "std/fs".into(), is_intrinsic: true },
            StdSymbol { name: "remove".into(), module: "std/fs".into(), is_intrinsic: true },
            StdSymbol { name: "copy".into(), module: "std/fs".into(), is_intrinsic: true },
            StdSymbol { name: "rename".into(), module: "std/fs".into(), is_intrinsic: true },
            StdSymbol { name: "metadata".into(), module: "std/fs".into(), is_intrinsic: true },
        ];
        let std_net = vec![
            StdSymbol { name: "TcpListener".into(), module: "std/net".into(), is_intrinsic: false },
            StdSymbol { name: "TcpStream".into(), module: "std/net".into(), is_intrinsic: false },
            StdSymbol { name: "UdpSocket".into(), module: "std/net".into(), is_intrinsic: false },
            StdSymbol { name: "SocketAddr".into(), module: "std/net".into(), is_intrinsic: false },
        ];
        let std_http = vec![
            StdSymbol { name: "Router".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "Request".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "Response".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "Headers".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "Cookie".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "Middleware".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "WebSocket".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "SSE".into(), module: "std/http".into(), is_intrinsic: false },
            StdSymbol { name: "serve".into(), module: "std/http".into(), is_intrinsic: true },
        ];
        let std_json = vec![
            StdSymbol { name: "Json".into(), module: "std/json".into(), is_intrinsic: false },
            StdSymbol { name: "json".into(), module: "std/json".into(), is_intrinsic: true },
            StdSymbol { name: "parse".into(), module: "std/json".into(), is_intrinsic: true },
            StdSymbol { name: "stringify".into(), module: "std/json".into(), is_intrinsic: true },
            StdSymbol { name: "Value".into(), module: "std/json".into(), is_intrinsic: false },
            StdSymbol { name: "Object".into(), module: "std/json".into(), is_intrinsic: false },
            StdSymbol { name: "Array".into(), module: "std/json".into(), is_intrinsic: false },
        ];
        let std_os = vec![StdSymbol { name: "os".into(), module: "std/os".into(), is_intrinsic: false }];
        let std_process = vec![
            StdSymbol { name: "process".into(), module: "std/process".into(), is_intrinsic: false },
            StdSymbol { name: "Command".into(), module: "std/process".into(), is_intrinsic: false },
            StdSymbol { name: "exit".into(), module: "std/process".into(), is_intrinsic: true },
            StdSymbol { name: "spawn".into(), module: "std/process".into(), is_intrinsic: true },
            StdSymbol { name: "wait".into(), module: "std/process".into(), is_intrinsic: true },
        ];
        let std_time = vec![
            StdSymbol { name: "time".into(), module: "std/time".into(), is_intrinsic: false },
            StdSymbol { name: "Instant".into(), module: "std/time".into(), is_intrinsic: false },
            StdSymbol { name: "Duration".into(), module: "std/time".into(), is_intrinsic: false },
            StdSymbol { name: "Timer".into(), module: "std/time".into(), is_intrinsic: false },
            StdSymbol { name: "sleep".into(), module: "std/time".into(), is_intrinsic: true },
        ];
        let std_path = vec![
            StdSymbol { name: "join".into(), module: "std/path".into(), is_intrinsic: true },
            StdSymbol { name: "extension".into(), module: "std/path".into(), is_intrinsic: true },
            StdSymbol { name: "filename".into(), module: "std/path".into(), is_intrinsic: true },
            StdSymbol { name: "normalize".into(), module: "std/path".into(), is_intrinsic: true },
            StdSymbol { name: "parent".into(), module: "std/path".into(), is_intrinsic: true },
        ];
        let std_env = vec![
            StdSymbol { name: "get".into(), module: "std/env".into(), is_intrinsic: true },
            StdSymbol { name: "set".into(), module: "std/env".into(), is_intrinsic: true },
            StdSymbol { name: "current_dir".into(), module: "std/env".into(), is_intrinsic: true },
            StdSymbol { name: "args".into(), module: "std/env".into(), is_intrinsic: true },
        ];
        let std_string = vec![
            StdSymbol { name: "split".into(), module: "std/string".into(), is_intrinsic: true },
            StdSymbol { name: "replace".into(), module: "std/string".into(), is_intrinsic: true },
            StdSymbol { name: "trim".into(), module: "std/string".into(), is_intrinsic: true },
            StdSymbol { name: "contains".into(), module: "std/string".into(), is_intrinsic: true },
            StdSymbol { name: "starts_with".into(), module: "std/string".into(), is_intrinsic: true },
            StdSymbol { name: "ends_with".into(), module: "std/string".into(), is_intrinsic: true },
            StdSymbol { name: "format".into(), module: "std/string".into(), is_intrinsic: true },
        ];
        let std_math = vec![
            StdSymbol { name: "sqrt".into(), module: "std/math".into(), is_intrinsic: true },
            StdSymbol { name: "pow".into(), module: "std/math".into(), is_intrinsic: true },
            StdSymbol { name: "sin".into(), module: "std/math".into(), is_intrinsic: true },
            StdSymbol { name: "cos".into(), module: "std/math".into(), is_intrinsic: true },
            StdSymbol { name: "abs".into(), module: "std/math".into(), is_intrinsic: true },
            StdSymbol { name: "random".into(), module: "std/math".into(), is_intrinsic: true },
        ];
        let std_io = vec![
            StdSymbol { name: "print".into(), module: "std/io".into(), is_intrinsic: true },
            StdSymbol { name: "println".into(), module: "std/io".into(), is_intrinsic: true },
            StdSymbol { name: "stdin".into(), module: "std/io".into(), is_intrinsic: false },
            StdSymbol { name: "stdout".into(), module: "std/io".into(), is_intrinsic: false },
            StdSymbol { name: "stderr".into(), module: "std/io".into(), is_intrinsic: false },
        ];
        let std_crypto = vec![StdSymbol { name: "crypto".into(), module: "std/crypto".into(), is_intrinsic: false }];
        let std_compress = vec![StdSymbol { name: "compress".into(), module: "std/compress".into(), is_intrinsic: false }];
        let std_log = vec![StdSymbol { name: "log".into(), module: "std/log".into(), is_intrinsic: false }];

        symbols.insert("std/alloc".into(), std_alloc);
        symbols.insert("std/fs".into(), std_fs);
        symbols.insert("std/net".into(), std_net);
        symbols.insert("std/http".into(), std_http);
        symbols.insert("std/json".into(), std_json);
        symbols.insert("std/os".into(), std_os);
        symbols.insert("std/process".into(), std_process);
        symbols.insert("std/time".into(), std_time);
        symbols.insert("std/path".into(), std_path);
        symbols.insert("std/env".into(), std_env);
        symbols.insert("std/string".into(), std_string);
        symbols.insert("std/math".into(), std_math);
        symbols.insert("std/io".into(), std_io);
        symbols.insert("std/crypto".into(), std_crypto);
        symbols.insert("std/compress".into(), std_compress);
        symbols.insert("std/log".into(), std_log);
        let std_iterator = vec![
            StdSymbol { name: "Iterator".into(), module: "std/iterator".into(), is_intrinsic: false },
            StdSymbol { name: "filter".into(), module: "std/iterator".into(), is_intrinsic: true },
            StdSymbol { name: "map".into(), module: "std/iterator".into(), is_intrinsic: true },
            StdSymbol { name: "take".into(), module: "std/iterator".into(), is_intrinsic: true },
            StdSymbol { name: "skip".into(), module: "std/iterator".into(), is_intrinsic: true },
            StdSymbol { name: "collect".into(), module: "std/iterator".into(), is_intrinsic: true },
        ];
        symbols.insert("std/iterator".into(), std_iterator);
        let std_async = vec![
            StdSymbol { name: "Future".into(), module: "std/async".into(), is_intrinsic: false },
            StdSymbol { name: "Task".into(), module: "std/async".into(), is_intrinsic: false },
            StdSymbol { name: "spawn".into(), module: "std/async".into(), is_intrinsic: true },
            StdSymbol { name: "await".into(), module: "std/async".into(), is_intrinsic: true },
            StdSymbol { name: "select".into(), module: "std/async".into(), is_intrinsic: true },
        ];
        symbols.insert("std/async".into(), std_async);
        let std_ai = vec![
            // Types
            StdSymbol { name: "Tensor".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Dataset".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Tokenizer".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Embedding".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "InferenceModel".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Vector".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Matrix".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "VectorStore".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "RAGEngine".into(), module: "std/ai".into(), is_intrinsic: false },
            // Providers
            StdSymbol { name: "OpenAI".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Anthropic".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "CustomAIProvider".into(), module: "std/ai".into(), is_intrinsic: false },
            // Phase 1: AI Standard Library
            StdSymbol { name: "MessageRole".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Message".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Tool".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ToolFunction".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ToolCall".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ChatOptions".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ChatResponse".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ChatChunk".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ChatStream".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Usage".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "EmbedOptions".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "EmbedResponse".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ImageOptions".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ImageResponse".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Image".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "STTOptions".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "STTResponse".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "TTSOptions".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "TTSResponse".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "AIConfig".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ResponseFormat".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "AnthropicMessage".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "ContentBlock".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "AnthropicResponse".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "AnthropicUsage".into(), module: "std/ai".into(), is_intrinsic: false },
            StdSymbol { name: "Model".into(), module: "std/ai".into(), is_intrinsic: false },
            // Functions
            StdSymbol { name: "chat".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "stream".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "embedding".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "embed".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "image".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "generate_image".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "transcribe".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "speech".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "generate_speech".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "claude".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "models".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "model_info".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "config".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "set_api_key".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "set_base_url".into(), module: "std/ai".into(), is_intrinsic: true },
            // Message helpers
            StdSymbol { name: "user_message".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "system_message".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "assistant_message".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "tool_message".into(), module: "std/ai".into(), is_intrinsic: true },
            // Tool helpers
            StdSymbol { name: "tool".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "tool_call_result".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "weather_tool".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "calculator_tool".into(), module: "std/ai".into(), is_intrinsic: true },
            StdSymbol { name: "search_tool".into(), module: "std/ai".into(), is_intrinsic: true },
            // Embedding helpers
            StdSymbol { name: "get_embedding_vector".into(), module: "std/ai".into(), is_intrinsic: true },
        ];
        symbols.insert("std/ai".into(), std_ai);

        Self { symbols }
    }

    pub fn is_stdlib_module(&self, path: &str) -> bool {
        self.symbols.contains_key(path)
    }

    pub fn get_module_symbols(&self, path: &str) -> Option<&[StdSymbol]> {
        self.symbols.get(path).map(|v| v.as_slice())
    }
}
