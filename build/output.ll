; ModuleID = 'main_module'
target triple = "aarch64-apple-darwin"

declare i32 @puts(i8*)
declare i8* @malloc(i64)
declare void @free(i8*)

define void @main() {
bb_0:
  %r0 = call i64 @println(i64 0)
  ret void
}

