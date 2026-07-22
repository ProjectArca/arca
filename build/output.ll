; ModuleID = 'main_module'
target triple = "aarch64-apple-darwin"

declare i32 @puts(i8*)
declare i8* @malloc(i64)
declare void @free(i8*)

define void @main() {
bb_0:
  %r0 = alloca i64
  %r1 = call i64 @fs_exists(i64 0)
  store i64 %r1, i64* %r0
  %r2 = load i64, i64* %r0
  %r3 = call i64 @println(i64 %r2)
  %r4 = alloca i64
  %r5 = call i64 @fs_remove(i64 0)
  store i64 %r5, i64* %r4
  %r6 = load i64, i64* %r4
  %r7 = call i64 @println(i64 %r6)
  ret void
}

