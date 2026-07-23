//! Type checking environment and symbol scope stack.

use crate::types::{FnType, PrimitiveType, Type};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Scope {
    pub bindings: HashMap<String, Type>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    scopes: Vec<Scope>,
    pub structs: HashMap<String, Type>,
    pub functions: HashMap<String, FnType>,
    pub current_struct: Option<String>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![Scope::new()],
            structs: HashMap::new(),
            functions: HashMap::new(),
            current_struct: None,
        };
        env.register_primitives();
        env
    }

    fn register_primitives(&mut self) {
        let mut arena_methods = HashMap::new();
        arena_methods.insert(
            "new".into(),
            FnType {
                params: Vec::new(),
                return_type: Box::new(Type::Struct {
                    name: "Arena".into(),
                    fields: HashMap::new(),
                    methods: HashMap::new(),
                }),
            },
        );

        let arena_struct = Type::Struct {
            name: "Arena".into(),
            fields: HashMap::new(),
            methods: arena_methods,
        };

        self.structs.insert("Arena".into(), arena_struct);

        let void_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };

        self.functions.insert("generateTable".into(), void_fn.clone());
        self.functions.insert("process".into(), void_fn.clone());
        self.functions.insert("println".into(), void_fn.clone());
        self.functions.insert("print".into(), void_fn.clone());
        self.functions.insert("panic".into(), void_fn.clone());
        self.functions.insert("assert".into(), void_fn.clone());

        let int_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        self.functions.insert("serve".into(), int_fn.clone());
        self.functions.insert("arca_std_http_serve".into(), int_fn.clone());

        let time_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("Instant.now".into(), time_fn.clone());
        self.functions.insert("now".into(), time_fn.clone());
        self.functions.insert("elapsed_ms".into(), time_fn.clone());
        self.functions.insert("elapsed_ns".into(), time_fn.clone());

        // Result/Option constructors
        let i64_to_i64 = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("Ok".into(), i64_to_i64.clone());
        self.functions.insert("Err".into(), i64_to_i64.clone());
        self.functions.insert("Some".into(), i64_to_i64.clone());
        self.functions.insert("arca_result_ok".into(), i64_to_i64.clone());
        self.functions.insert("arca_result_err".into(), i64_to_i64.clone());
        self.functions.insert("arca_option_some".into(), i64_to_i64.clone());
        self.functions.insert("__arca_throw".into(), i64_to_i64.clone());

        let i64_to_i32 = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        self.functions.insert("arca_result_is_ok".into(), i64_to_i32.clone());
        self.functions.insert("arca_option_is_some".into(), i64_to_i32.clone());
        self.functions.insert("arca_result_unwrap".into(), i64_to_i64.clone());

        let void_to_i64 = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("__arca_get_last_error".into(), void_to_i64.clone());

        let void_to_void = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("__arca_clear_last_error".into(), void_to_void.clone());

        // std/math
        let math_i64_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("sqrt".into(), math_i64_fn.clone());
        self.functions.insert("sin".into(), math_i64_fn.clone());
        self.functions.insert("cos".into(), math_i64_fn.clone());
        self.functions.insert("abs".into(), math_i64_fn.clone());
        let math_i64_i64_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("pow".into(), math_i64_i64_fn.clone());
        let math_void_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("rand".into(), math_void_fn);

        // std/math extras
        let math_i64_i64_i64_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("min".into(), math_i64_i64_fn.clone());
        self.functions.insert("max".into(), math_i64_i64_fn.clone());
        self.functions.insert("clamp".into(), math_i64_i64_i64_fn);
        self.functions.insert("floor".into(), math_i64_fn.clone());
        self.functions.insert("ceil".into(), math_i64_fn.clone());
        self.functions.insert("round".into(), math_i64_fn.clone());
        self.functions.insert("log".into(), math_i64_fn.clone());
        self.functions.insert("exp".into(), math_i64_fn.clone());
        let random_range_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("random_range".into(), random_range_fn);

        // std/os module
        let string_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("compress".into(), string_fn.clone());
        self.functions.insert("sha256".into(), string_fn.clone());

        // std/fs operations
        let string_to_i32 = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        let string_string_to_i32 = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        let string_string_to_string = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("file_read".into(), string_fn.clone());
        self.functions.insert("file_write".into(), string_string_to_i32.clone());
        self.functions.insert("file_append".into(), string_string_to_i32.clone());
        self.functions.insert("file_copy".into(), string_string_to_i32.clone());
        self.functions.insert("file_rename".into(), string_string_to_i32.clone());
        self.functions.insert("file_remove".into(), string_to_i32.clone());
        self.functions.insert("file_mkdir".into(), string_to_i32.clone());
        self.functions.insert("file_exists".into(), string_to_i32.clone());

        // Namespaced API: File.*
        let string_string_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("File.read".into(), string_fn.clone());
        self.functions.insert("File.write".into(), string_string_to_i32.clone());
        self.functions.insert("File.copy".into(), string_string_to_i32.clone());
        self.functions.insert("File.exists".into(), string_to_i32.clone());
        self.functions.insert("File.remove".into(), string_to_i32.clone());
        self.functions.insert("File.mkdir".into(), string_to_i32.clone());
        self.functions.insert("File.rename".into(), string_string_to_i32.clone());
        self.functions.insert("File.append".into(), string_string_to_i32.clone());

        // Namespaced API: Path.*
        self.functions.insert("Path.join".into(), string_string_fn.clone());
        self.functions.insert("Path.parent".into(), string_fn.clone());
        self.functions.insert("Path.filename".into(), string_fn.clone());
        self.functions.insert("Path.extension".into(), string_fn.clone());

        // Namespaced API: Result.*
        let i64_to_i64 = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("Result.ok".into(), i64_to_i64.clone());
        self.functions.insert("Result.err".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("Result.is_ok".into(), i64_to_i64.clone());
        self.functions.insert("Result.unwrap".into(), i64_to_i64.clone());

        // Namespaced API: Option.*
        self.functions.insert("Option.is_some".into(), i64_to_i64.clone());
        self.functions.insert("Option.unwrap".into(), i64_to_i64.clone());

        // std/encoding
        let int_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        self.functions.insert("hex_encode".into(), string_fn.clone());
        self.functions.insert("urlencode".into(), string_fn.clone());
        self.functions.insert("urldecode".into(), string_fn.clone());

        // std/net
        self.functions.insert("tcp_listen".into(), int_fn.clone());
        self.functions.insert("tcp_accept".into(), int_fn.clone());
        self.functions.insert("tcp_recv".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });

        // std/json method wrappers
        self.functions.insert("Json.value".into(), string_string_to_string.clone());
        self.functions.insert("Json.pretty".into(), string_fn.clone());
        self.functions.insert("Json.object".into(), string_fn.clone());
        self.functions.insert("Json.array".into(), string_fn.clone());
        let info_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("info".into(), info_fn.clone());

        // log module extras
        self.functions.insert("warn".into(), info_fn.clone());
        self.functions.insert("error".into(), info_fn.clone());
        self.functions.insert("debug".into(), info_fn.clone());

        // crypto extras
        self.functions.insert("random_bytes".into(), string_fn.clone());
        self.functions.insert("aes_gcm_encrypt".into(), string_fn.clone());

        // compress extras
        let decompress_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("decompress".into(), decompress_fn);

        // json
        let _string_to_void_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("Json.stringify".into(), string_fn.clone());

        // random
        self.functions.insert("Random.next_i64".into(), time_fn.clone());
        let uuid_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("Random.uuid_v4".into(), uuid_fn.clone());

        // fs
        self.functions.insert("File.open".into(), string_fn.clone());
        let file_close_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        self.functions.insert("File.close".into(), file_close_fn);

        // net
        self.functions.insert("TcpListener.bind".into(), int_fn.clone());

        // sync (channel already has handlers)
        self.functions.insert("Channel.new".into(), int_fn.clone());

        // task
        self.functions.insert("Task.yield_now".into(), void_to_void.clone());

        // http
        let response_text_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("Response.ok".into(), time_fn.clone());
        self.functions.insert("Response.text".into(), response_text_fn.clone());
        self.functions.insert("Response.html".into(), response_text_fn.clone());
        self.functions.insert("Response.json".into(), response_text_fn.clone());
        self.functions.insert("Response.not_found".into(), time_fn.clone());

        let request_string_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("Request.param".into(), request_string_fn.clone());
        self.functions.insert("Request.query".into(), request_string_fn.clone());
        self.functions.insert("Request.header".into(), request_string_fn.clone());
        let os_arch = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("arch".into(), os_arch.clone());
        self.functions.insert("cpu_count".into(), time_fn.clone());
        self.functions.insert("env".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });

        // std/time
        let sleep_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("sleep".into(), sleep_fn);

        // std/env
        self.functions.insert("env_get".into(), string_fn.clone());
        self.functions.insert("env_set".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("current_dir".into(), uuid_fn.clone());

        // std/io
        self.functions.insert("stdin_read_line".into(), uuid_fn.clone());
        self.functions.insert("stdout_write".into(), info_fn.clone());
        self.functions.insert("stderr_write".into(), info_fn.clone());

        // std/fs
        let fs_open_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("File.open".into(), fs_open_fn);
        self.functions.insert("fs_exists".into(), string_fn.clone());
        self.functions.insert("fs_remove".into(), string_fn.clone());
        self.functions.insert("fs_rename".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        });
        self.functions.insert("fs_copy".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        });
        self.functions.insert("fs_metadata".into(), string_fn.clone());

        // std/path
        self.functions.insert("path_extension".into(), string_fn.clone());
        self.functions.insert("path_filename".into(), string_fn.clone());
        self.functions.insert("path_parent".into(), string_fn.clone());
        let path_join_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("path_join".into(), path_join_fn);
        self.functions.insert("path_normalize".into(), string_fn.clone());

        // std/process
        let exit_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("exit".into(), exit_fn);

        // std/json
        self.functions.insert("json_stringify".into(), string_fn.clone());

        // std/string extras
        let string_string_i32_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::I32)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        let string_3_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        let string_2_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("arca_str_split".into(), string_string_i32_fn.clone());
        self.functions.insert("split".into(), string_string_i32_fn.clone());
        self.functions.insert("__arca_str_find".into(), string_string_i32_fn.clone());
        self.functions.insert("__arca_str_count".into(), string_string_i32_fn);
        self.functions.insert("arca_str_replace".into(), string_3_fn.clone());
        self.functions.insert("replace".into(), string_3_fn);
        self.functions.insert("arca_str_format".into(), string_2_fn.clone());
        self.functions.insert("format".into(), string_2_fn);
        self.functions.insert("arca_str_slice".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });
        self.functions.insert("arca_str_len".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });

        // std/fs extras
        self.functions.insert("arca_fs_mkdir".into(), i64_to_i32.clone());
        self.functions.insert("arca_fs_rmdir".into(), i64_to_i32.clone());
        self.functions.insert("arca_fs_read_dir".into(), uuid_fn.clone());

        // std/process extras
        self.functions.insert("arca_process_command".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("arca_process_spawn".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("arca_process_wait".into(), i64_to_i32.clone());

        // std/net extras
        self.functions.insert("arca_tcp_connect".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::I32)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        });
        self.functions.insert("arca_udp_bind".into(), int_fn.clone());

        // std/json extras
        self.functions.insert("arca_json_parse".into(), string_fn.clone());
        self.functions.insert("parse".into(), string_fn.clone());

        // Vec runtime helpers (for array literal support)
        let i64_i64_to_i64 = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("arca_vec_len".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("arca_vec_get".into(), i64_i64_to_i64.clone());
        self.functions.insert("arca_vec_push".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        });

        // ===== PATCH 1: std/string methods =====
        let _void_fn = FnType { params: vec![], return_type: Box::new(Type::Primitive(PrimitiveType::Void)) };
        let string_void_to_i64 = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        let string_i64_to_string = FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("arca_str_len".into(), string_void_to_i64.clone());
        self.functions.insert("__arca_str_is_empty".into(), string_void_to_i64.clone());
        self.functions.insert("__arca_str_at".into(), string_i64_to_string.clone());
        self.functions.insert("__arca_str_lines".into(), string_fn.clone());
        self.functions.insert("__arca_str_lower".into(), string_fn.clone());
        self.functions.insert("__arca_str_upper".into(), string_fn.clone());
        self.functions.insert("__arca_str_repeat".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });
        self.functions.insert("__arca_hostname".into(), string_fn.clone());
        self.functions.insert("__arca_username".into(), string_fn.clone());

        // ===== PATCH 2: std/collections method wrappers =====
        // These are method-style calls that get dispatched in the backend
        let i64_to_void = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("vec_push_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        });
        self.functions.insert("vec_get_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("vec_pop_m".into(), i64_to_i64.clone());
        self.functions.insert("vec_insert_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        });
        self.functions.insert("vec_remove_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        });
        self.functions.insert("vec_clear_m".into(), i64_to_void.clone());
        self.functions.insert("map_set_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        });
        self.functions.insert("map_has_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });
        self.functions.insert("set_insert_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        });

        // ===== PATCH 8: std/http method wrappers =====
        self.functions.insert("router_post_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });
        self.functions.insert("router_get_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });
        self.functions.insert("router_put_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });
        self.functions.insert("router_delete_m".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::I64), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });

        // Standard library module bindings
        self.insert_var("serve".into(), Type::Unknown);
        self.insert_var("log".into(), Type::Unknown);
        self.insert_var("crypto".into(), Type::Unknown);
        self.insert_var("gzip".into(), Type::Unknown);
        self.insert_var("zstd".into(), Type::Unknown);
        self.insert_var("math".into(), Type::Unknown);
        self.insert_var("mem".into(), Type::Unknown);
        self.insert_var("hash".into(), Type::Unknown);
        self.insert_var("json".into(), Type::Unknown);
        self.insert_var("os".into(), Type::Unknown);
        self.insert_var("process".into(), Type::Unknown);
        self.insert_var("time".into(), Type::Unknown);
        self.insert_var("Process".into(), Type::Unknown);
        self.insert_var("Command".into(), Type::Unknown);
        self.insert_var("Directory".into(), Type::Unknown);
        self.insert_var("TcpStream".into(), Type::Unknown);
        self.insert_var("UdpSocket".into(), Type::Unknown);
        self.insert_var("SocketAddr".into(), Type::Unknown);
        self.insert_var("Headers".into(), Type::Unknown);
        self.insert_var("Cookie".into(), Type::Unknown);
        self.insert_var("Middleware".into(), Type::Unknown);
        self.insert_var("WebSocket".into(), Type::Unknown);
        self.insert_var("SSE".into(), Type::Unknown);
        self.insert_var("Json".into(), Type::Unknown);
        self.insert_var("Path".into(), Type::Unknown);
        self.insert_var("Result".into(), Type::Unknown);
        self.insert_var("Option".into(), Type::Unknown);
        self.insert_var("OpenAI".into(), Type::Unknown);
        self.insert_var("Anthropic".into(), Type::Unknown);
        self.insert_var("CustomAIProvider".into(), Type::Unknown);
        self.insert_var("ai".into(), Type::Unknown);

        // std/ai structs
        let tensor_struct = Type::Struct {
            name: "Tensor".into(),
            fields: vec![
                ("shape".into(), Type::Primitive(PrimitiveType::String)),
                ("data_ptr".into(), Type::Primitive(PrimitiveType::I64)),
            ].into_iter().collect(),
            methods: HashMap::new(),
        };
        self.structs.insert("Tensor".into(), tensor_struct);

        let dataset_struct = Type::Struct {
            name: "Dataset".into(),
            fields: vec![
                ("format".into(), Type::Primitive(PrimitiveType::String)),
                ("path".into(), Type::Primitive(PrimitiveType::String)),
            ].into_iter().collect(),
            methods: HashMap::new(),
        };
        self.structs.insert("Dataset".into(), dataset_struct);

        let tokenizer_struct = Type::Struct {
            name: "Tokenizer".into(),
            fields: vec![
                ("kind".into(), Type::Primitive(PrimitiveType::String)),
            ].into_iter().collect(),
            methods: HashMap::new(),
        };
        self.structs.insert("Tokenizer".into(), tokenizer_struct);

        let mut openai_methods = HashMap::new();
        openai_methods.insert("chat".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });

        let openai_struct = Type::Struct {
            name: "OpenAI".into(),
            fields: vec![
                ("api_key".into(), Type::Primitive(PrimitiveType::String)),
                ("model".into(), Type::Primitive(PrimitiveType::String)),
            ].into_iter().collect(),
            methods: openai_methods,
        };
        self.structs.insert("OpenAI".into(), openai_struct.clone());

        let vector_store_struct = Type::Struct {
            name: "VectorStore".into(),
            fields: vec![
                ("handle".into(), Type::Primitive(PrimitiveType::I64)),
            ].into_iter().collect(),
            methods: HashMap::new(),
        };
        self.structs.insert("VectorStore".into(), vector_store_struct.clone());

        let mut rag_methods = HashMap::new();
        rag_methods.insert("query".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });

        let rag_engine_struct = Type::Struct {
            name: "RAGEngine".into(),
            fields: vec![
                ("handle".into(), Type::Primitive(PrimitiveType::I64)),
            ].into_iter().collect(),
            methods: rag_methods,
        };
        self.structs.insert("RAGEngine".into(), rag_engine_struct.clone());

        self.functions.insert("OpenAI.chat".into(), FnType {
            params: vec![openai_struct, Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });
        self.functions.insert("VectorStore.connect".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(vector_store_struct.clone()),
        });
        self.functions.insert("RAGEngine.new".into(), FnType {
            params: vec![vector_store_struct, Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(rag_engine_struct.clone()),
        });
        self.functions.insert("RAGEngine.query".into(), FnType {
            params: vec![rag_engine_struct, Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        });

        // std/collections structs — all handle wrappers with no methods
        let handle_struct = |name: &str| -> Type {
            Type::Struct {
                name: name.into(),
                fields: vec![("handle".into(), Type::Primitive(PrimitiveType::I64))].into_iter().collect(),
                methods: HashMap::new(),
            }
        };
        self.structs.insert("Vec".into(), handle_struct("Vec"));
        self.structs.insert("HashMap".into(), handle_struct("HashMap"));
        self.structs.insert("HashSet".into(), handle_struct("HashSet"));
        self.structs.insert("Queue".into(), handle_struct("Queue"));
        self.structs.insert("Deque".into(), handle_struct("Deque"));
        self.structs.insert("BinaryHeap".into(), handle_struct("BinaryHeap"));
        self.structs.insert("LinkedList".into(), handle_struct("LinkedList"));

        // Iterator struct with methods
        let i64_ty = Type::Primitive(PrimitiveType::I64);
        let string_ty = Type::Primitive(PrimitiveType::String);
        let iterator_ty = Type::Struct {
            name: "Iterator".into(),
            fields: vec![("handle".into(), i64_ty.clone())].into_iter().collect(),
            methods: vec![
                ("filter".into(), FnType {
                    params: vec![i64_ty.clone()],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("map".into(), FnType {
                    params: vec![i64_ty.clone()],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("take".into(), FnType {
                    params: vec![i64_ty.clone()],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("skip".into(), FnType {
                    params: vec![i64_ty.clone()],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("collect".into(), FnType {
                    params: vec![],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("reduce".into(), FnType {
                    params: vec![i64_ty.clone(), i64_ty.clone()],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("enumerate".into(), FnType {
                    params: vec![],
                    return_type: Box::new(i64_ty.clone()),
                }),
            ].into_iter().collect(),
        };
        self.structs.insert("Iterator".into(), iterator_ty);

        // Channel struct with methods
        let channel_ty = Type::Struct {
            name: "Channel".into(),
            fields: vec![].into_iter().collect(),
            methods: vec![
                ("new".into(), FnType {
                    params: vec![i64_ty.clone()],
                    return_type: Box::new(i64_ty.clone()),
                }),
                ("send".into(), FnType {
                    params: vec![i64_ty.clone()],
                    return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
                }),
                ("recv".into(), FnType {
                    params: vec![],
                    return_type: Box::new(i64_ty.clone()),
                }),
            ].into_iter().collect(),
        };
        self.structs.insert("Channel".into(), channel_ty);

        // std/http structs
        self.structs.insert("Request".into(), Type::Struct {
            name: "Request".into(),
            fields: vec![
                ("method".into(), string_ty.clone()),
                ("path".into(), string_ty.clone()),
                ("url".into(), string_ty.clone()),
            ].into_iter().collect(),
            methods: vec![
                ("param".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
                ("query".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
                ("header".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
            ].into_iter().collect(),
        });

        self.structs.insert("Response".into(), Type::Struct {
            name: "Response".into(),
            fields: vec![
                ("status".into(), Type::Primitive(PrimitiveType::I32)),
                ("content_type".into(), string_ty.clone()),
                ("body".into(), string_ty.clone()),
            ].into_iter().collect(),
            methods: HashMap::new(),
        });

        self.structs.insert("Router".into(), Type::Struct {
            name: "Router".into(),
            fields: vec![
                ("prefix".into(), string_ty.clone()),
            ].into_iter().collect(),
            methods: vec![
                ("post".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
                ("get".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
                ("put".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
                ("delete".into(), FnType {
                    params: vec![string_ty.clone()],
                    return_type: Box::new(string_ty.clone()),
                }),
            ].into_iter().collect(),
        });

        self.structs.insert("Cookie".into(), Type::Struct {
            name: "Cookie".into(),
            fields: vec![
                ("name".into(), string_ty.clone()),
                ("value".into(), string_ty.clone()),
            ].into_iter().collect(),
            methods: vec![
                ("to_header".into(), FnType {
                    params: vec![],
                    return_type: Box::new(string_ty.clone()),
                }),
            ].into_iter().collect(),
        });

        self.structs.insert("Middleware".into(), Type::Struct {
            name: "Middleware".into(),
            fields: vec![
                ("name".into(), string_ty.clone()),
            ].into_iter().collect(),
            methods: HashMap::new(),
        });

        self.insert_var("VectorStore".into(), Type::Unknown);
        self.insert_var("RAGEngine".into(), Type::Unknown);
        self.insert_var("Tensor".into(), Type::Unknown);
        self.insert_var("Dataset".into(), Type::Unknown);
        self.insert_var("Tokenizer".into(), Type::Unknown);
        self.insert_var("Embedding".into(), Type::Unknown);
        self.insert_var("InferenceModel".into(), Type::Unknown);
        self.insert_var("Vector".into(), Type::Unknown);
        self.insert_var("Matrix".into(), Type::Unknown);
        self.insert_var("Future".into(), Type::Unknown);
        self.insert_var("Task".into(), Type::Unknown);
        self.insert_var("Value".into(), Type::Unknown);
        self.insert_var("Object".into(), Type::Unknown);
        self.insert_var("Instant".into(), Type::Unknown);
        self.insert_var("Duration".into(), Type::Unknown);
        self.insert_var("Timer".into(), Type::Unknown);
        self.insert_var("ArenaAllocator".into(), Type::Unknown);
        self.insert_var("Pool".into(), Type::Unknown);
        self.insert_var("Arena".into(), Type::Unknown);
        self.insert_var("File".into(), Type::Unknown);
        self.insert_var("TcpListener".into(), Type::Unknown);
        self.insert_var("Array".into(), Type::Unknown);
        self.insert_var("Map".into(), Type::Unknown);
        self.insert_var("Set".into(), Type::Unknown);

        // FFI / Native interop namespace
        self.insert_var("c".into(), Type::Unknown);
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert_var(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name, ty);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.bindings.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub fn lookup_type_annotation(&self, type_str: &str) -> Type {
        match type_str {
            "i8" => Type::Primitive(PrimitiveType::I8),
            "i16" => Type::Primitive(PrimitiveType::I16),
            "i32" => Type::Primitive(PrimitiveType::I32),
            "i64" => Type::Primitive(PrimitiveType::I64),
            "u8" => Type::Primitive(PrimitiveType::U8),
            "u16" => Type::Primitive(PrimitiveType::U16),
            "u32" => Type::Primitive(PrimitiveType::U32),
            "u64" => Type::Primitive(PrimitiveType::U64),
            "f32" => Type::Primitive(PrimitiveType::F32),
            "f64" => Type::Primitive(PrimitiveType::F64),
            "bool" => Type::Primitive(PrimitiveType::Bool),
            "string" => Type::Primitive(PrimitiveType::String),
            "char" => Type::Primitive(PrimitiveType::Char),
            "void" => Type::Primitive(PrimitiveType::Void),
            "Self" => {
                if let Some(sname) = &self.current_struct {
                    if let Some(st) = self.structs.get(sname) {
                        st.clone()
                    } else {
                        Type::Unknown
                    }
                } else {
                    Type::Unknown
                }
            }
            "c_void_ptr" => Type::Primitive(PrimitiveType::Void),
            custom => {
                if let Some(st) = self.structs.get(custom) {
                    st.clone()
                } else {
                    Type::Unknown
                }
            }
        }
    }
}
